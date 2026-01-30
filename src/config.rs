use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::ui::Ui;

const REPO_URL: &str = "https://github.com/tahayvr/omarchy-iso";
const OMARCHY_BASE_PKGS_URL: &str = "https://raw.githubusercontent.com/basecamp/omarchy/refs/heads/master/install/omarchy-base.packages";

// Guard to kill child process group on drop
struct ProcessGuard(Option<Child>);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.take() {
            if let Some(id) = child.id() {
                // Kill the entire process group using system 'kill'
                let _ = std::process::Command::new("kill")
                    .args(["-9", &format!("-{}", id)])
                    .output();
            }
        }
    }
}

pub struct Config {
    pub work_dir: PathBuf,
    pub aur_packages: Vec<String>,
    pub selected_aur: Vec<bool>,
    pub official_packages: Vec<String>,
    pub selected_official: Vec<bool>,
    pub omarchy_packages: Vec<String>,
    pub selected_ignored: Vec<bool>,
    pub dotfiles: Vec<String>,
    pub selected_dotfiles: Vec<bool>,
    pub home_dotfiles: Vec<String>,
    pub selected_home_dotfiles: Vec<bool>,
    pub iso_path: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        let work_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("omarchyiso_build");

        Self {
            work_dir,
            aur_packages: Vec::new(),
            selected_aur: Vec::new(),
            official_packages: Vec::new(),
            selected_official: Vec::new(),
            omarchy_packages: Vec::new(),
            selected_ignored: Vec::new(),
            dotfiles: Vec::new(),
            selected_dotfiles: Vec::new(),
            home_dotfiles: Vec::new(),
            selected_home_dotfiles: Vec::new(),
            iso_path: None,
        }
    }

    pub async fn clean_work_dir(&self) -> Result<()> {
        if self.work_dir.exists() {
            // Attempt to unmount anything just in case
            let _ = AsyncCommand::new("umount")
                .arg("-R")
                .arg(&self.work_dir)
                .output()
                .await;
            
            // Try to remove the directory
            if let Err(_e) = fs::remove_dir_all(&self.work_dir).await {
                // If native removal fails (likely permission issues from Docker artifacts),
                // try to use Docker to remove it since Docker created the mess.
                if let Some(parent) = self.work_dir.parent() {
                    let dir_name = self.work_dir.file_name().unwrap().to_str().unwrap();
                    let parent_lossy = parent.to_string_lossy();
                    
                    if let Ok(mut child) = AsyncCommand::new("docker")
                        .args([
                            "run", "--rm", "--privileged",
                            "-v", &format!("{}:/work", parent_lossy),
                            "alpine", "rm", "-rf", &format!("/work/{}", dir_name)
                        ])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        let _ = child.wait().await;
                    }
                }
                
                // Final retry with standard remove in case docker fixed it or it was a temporary lock
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if self.work_dir.exists() {
                     fs::remove_dir_all(&self.work_dir).await.context("Failed to clean build directory. You may need to run with sudo or manually remove 'omarchyiso_build'.")?;
                }
            }
        }
        Ok(())
    }

    pub async fn setup(&self) -> Result<()> {
        // Remove existing build directory
        self.clean_work_dir().await?;

        // Clone repository
        let output = AsyncCommand::new("git")
            .args(["clone", "-q", REPO_URL, self.work_dir.to_str().unwrap()])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Failed to clone repository");
        }

        Ok(())
    }

    pub async fn scan_aur_packages(&mut self) -> Result<()> {
        let output = AsyncCommand::new("pacman").args(["-Qmq"]).output().await?;

        if output.status.success() {
            let packages = String::from_utf8_lossy(&output.stdout);
            self.aur_packages = packages
                .lines()
                .filter(|l| !l.is_empty())
                .map(|s| s.to_string())
                .collect();
            self.selected_aur = vec![false; self.aur_packages.len()];
        }

        Ok(())
    }

    pub async fn scan_official_packages(&mut self) -> Result<()> {
        let output = AsyncCommand::new("sh")
            .arg("-c")
            .arg("expac -S \"%r %n\" $(pacman -Qqe) | grep -vE \"^(core|multilib|omarchy)\" | awk '{print $2}'")
            .output()
            .await?;

        if output.status.success() {
            let packages = String::from_utf8_lossy(&output.stdout);
            self.official_packages = packages
                .lines()
                .filter(|l| !l.is_empty())
                .map(|s| s.to_string())
                .collect();
            self.selected_official = vec![false; self.official_packages.len()];
        }

        Ok(())
    }

    pub async fn scan_omarchy_packages(&mut self) -> Result<()> {
        let output = AsyncCommand::new("curl")
            .args(["-s", OMARCHY_BASE_PKGS_URL])
            .output()
            .await?;

        if output.status.success() {
            let packages = String::from_utf8_lossy(&output.stdout);
            self.omarchy_packages = packages
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|s| s.to_string())
                .collect();
            self.selected_ignored = vec![false; self.omarchy_packages.len()];
        }

        Ok(())
    }

    pub async fn scan_dotfiles(&mut self) -> Result<()> {
        let user_home = std::env::var("SUDO_USER")
            .ok()
            .and_then(|u| dirs::home_dir().map(|h| h.parent().unwrap().join(&u)))
            .or_else(dirs::home_dir)
            .context("Could not determine home directory")?;

        let config_dir = user_home.join(".config");

        if !config_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&config_dir).await?;
        let mut items = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    items.push(format!("{}/", name)); // Add trailing slash for folders
                } else if file_type.is_file() {
                    items.push(name.to_string());
                }
            }
        }

        items.sort();
        self.dotfiles = items;
        self.selected_dotfiles = vec![false; self.dotfiles.len()];

        Ok(())
    }

    pub async fn scan_home_dotfiles(&mut self) -> Result<()> {
        let user_home = std::env::var("SUDO_USER")
            .ok()
            .and_then(|u| dirs::home_dir().map(|h| h.parent().unwrap().join(&u)))
            .or_else(dirs::home_dir)
            .context("Could not determine home directory")?;

        if !user_home.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&user_home).await?;
        let mut items = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                // Only include dotfiles (files starting with .)
                if !name.starts_with('.') {
                    continue;
                }
                
                // Skip .config directory (handled separately)
                if name == ".config" {
                    continue;
                }
                
                // Skip common directories we don't want to include
                if matches!(name, ".cache" | ".local" | ".mozilla" | ".ssh" | ".gnupg") {
                    continue;
                }

                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    items.push(format!("{}/", name)); // Add slash for folders
                } else if file_type.is_file() {
                    items.push(name.to_string());
                }
            }
        }

        items.sort();
        self.home_dotfiles = items;
        self.selected_home_dotfiles = vec![false; self.home_dotfiles.len()];

        Ok(())
    }

    pub fn finalize_aur_selection(&mut self, _ui: &Ui) {
        // UI will update selected_aur based on user interaction
    }

    pub fn finalize_official_selection(&mut self, _ui: &Ui) {
        // UI will update selected_official based on user interaction
    }

    pub fn finalize_ignored_selection(&mut self, _ui: &Ui) {
        // UI will update selected_ignored based on user interaction
    }

    pub fn finalize_dotfiles_selection(&mut self, _ui: &Ui) {
        // UI will update selected_dotfiles based on user interaction
    }

    pub fn finalize_home_dotfiles_selection(&mut self, _ui: &Ui) {
        // UI will update selected_home_dotfiles based on user interaction
    }

    pub fn get_selected_aur(&self) -> Vec<String> {
        self.aur_packages
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_aur.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn get_selected_official(&self) -> Vec<String> {
        self.official_packages
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_official.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn get_selected_ignored(&self) -> Vec<String> {
        self.omarchy_packages
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_ignored.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn get_selected_dotfiles(&self) -> Vec<String> {
        self.dotfiles
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_dotfiles.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| {
                // Remove trailing slash
                let item = p.trim_end_matches('/');
                format!(".config/{}", item)
            })
            .collect()
    }

    pub fn get_selected_home_dotfiles(&self) -> Vec<String> {
        self.home_dotfiles
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_home_dotfiles.get(*i).copied().unwrap_or(false))
            .map(|(_, p)| {
                // Remove trailing slash
                p.trim_end_matches('/').to_string()
            })
            .collect()
    }

    pub async fn build_iso(&mut self, output_tx: mpsc::UnboundedSender<String>) -> Result<()> {
        // Setup
        output_tx
            .send("Cloning Omarchy ISO repository...".to_string())
            .ok();
        self.setup()
            .await
            .context("Failed to clone Omarchy ISO repository")?;
        output_tx
            .send("✓ Repository cloned successfully".to_string())
            .ok();

        // Write package files
        output_tx
            .send("Writing package configuration files...".to_string())
            .ok();
        self.write_package_files()
            .await
            .context("Failed to write package files")?;
        output_tx.send("✓ Package files written".to_string()).ok();

        // Apply dotfiles
        let selected_dotfiles = self.get_selected_dotfiles();
        if !selected_dotfiles.is_empty() {
            output_tx
                .send(format!(
                    "Copying {} .config dotfile(s)...",
                    selected_dotfiles.len()
                ))
                .ok();
            self.apply_dotfiles()
                .await
                .context("Failed to apply dotfiles")?;
            output_tx.send("✓ .config dotfiles copied".to_string()).ok();
        }

        // Apply home directory dotfiles
        let selected_home_dotfiles = self.get_selected_home_dotfiles();
        if !selected_home_dotfiles.is_empty() {
            output_tx
                .send(format!(
                    "Copying {} home dotfile(s)...",
                    selected_home_dotfiles.len()
                ))
                .ok();
            self.apply_home_dotfiles()
                .await
                .context("Failed to apply home dotfiles")?;
            output_tx.send("✓ Home dotfiles copied".to_string()).ok();
        }

        // Execute build
        output_tx.send("".to_string()).ok();
        output_tx
            .send("Starting ISO build process...".to_string())
            .ok();
        output_tx
            .send("⏱  This will take 15-60 minutes".to_string())
            .ok();
        output_tx.send("".to_string()).ok();

        let mut child = AsyncCommand::new("./bin/omarchy-iso-make")
            .args(["--no-boot-offer", "--no-cache"])
            .current_dir(&self.work_dir)
            .process_group(0) // Create a new process group
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn omarchy-iso-make")?;

        // Capture output streams
        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        // Wrap child in guard to ensure kill on drop (cancellation)
        let mut child_guard = ProcessGuard(Some(child));

        // Stream stdout in real-time
        let tx_stdout = output_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx_stdout.send(line).is_err() {
                    break;
                }
            }
        });

        // Stream stderr in real-time
        let tx_stderr = output_tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx_stderr.send(format!("⚠ {}", line)).is_err() {
                    break;
                }
            }
        });

        // Wait for both streams to complete
        let _ = tokio::join!(stdout_handle, stderr_handle);

        // Wait for the process to complete
        let child = child_guard.0.as_mut().unwrap();
        let status = child
            .wait()
            .await
            .context("Failed to wait for build process")?;

        // Process completed, take out of guard
        let _ = child_guard.0.take();

        if !status.success() {
            output_tx.send("".to_string()).ok();
            output_tx.send("❌ Build failed!".to_string()).ok();

            // Cleanup on failure
            self.clean_work_dir().await.ok();

            anyhow::bail!("Build failed with exit code {:?}", status.code());
        }

        output_tx.send("".to_string()).ok();
        output_tx
            .send("✓ ISO build completed successfully".to_string())
            .ok();

        // Find and move ISO
        output_tx
            .send("Moving ISO to parent directory...".to_string())
            .ok();
        self.move_iso_to_parent()
            .await
            .context("Failed to move ISO to parent directory")?;
        output_tx.send("✓ ISO moved successfully".to_string()).ok();

        Ok(())
    }

    async fn write_package_files(&self) -> Result<()> {
        let builder_dir = self.work_dir.join("builder");
        fs::create_dir_all(&builder_dir).await?;

        // Write official packages
        let selected_official = self.get_selected_official();
        if !selected_official.is_empty() {
            let content = selected_official.join("\n");
            fs::write(builder_dir.join("custom-arch.packages"), content).await?;
        }

        // Write AUR packages
        let selected_aur = self.get_selected_aur();
        if !selected_aur.is_empty() {
            let content = selected_aur.join("\n");
            fs::write(builder_dir.join("custom-aur.packages"), content).await?;
        }

        // Write ignored packages
        let selected_ignored = self.get_selected_ignored();
        if !selected_ignored.is_empty() {
            let content = selected_ignored.join("\n");
            fs::write(builder_dir.join("custom.ignored"), content).await?;
        }

        Ok(())
    }

    async fn apply_dotfiles(&self) -> Result<()> {
        let selected = self.get_selected_dotfiles();
        if selected.is_empty() {
            return Ok(());
        }

        let user_home = std::env::var("SUDO_USER")
            .ok()
            .and_then(|u| dirs::home_dir().map(|h| h.parent().unwrap().join(&u)))
            .or_else(dirs::home_dir)
            .context("Could not determine home directory")?;

        let target_dir = self.work_dir.join("configs/airootfs/root/custom-config");
        fs::create_dir_all(&target_dir).await?;

        for item in selected {
            // Remove trailing slash if for folders
            let item_name = item.trim_end_matches('/');
            let src_path = user_home.join(item_name);
            // For destination, we want to strip the .config/ prefix so they are flat inside custom-config
            let clean_name = item_name.strip_prefix(".config/").unwrap_or(item_name);
            let dest_path = target_dir.join(clean_name);

            if !src_path.exists() {
                continue;
            }

            // Check if it's a directory or file
            let metadata = fs::metadata(&src_path).await?;
            if metadata.is_dir() {
                crate::utils::copy_dir_recursive(&src_path, &dest_path).await?;
            } else {
                fs::copy(&src_path, &dest_path).await?;
            }
        }

        Ok(())
    }

    async fn apply_home_dotfiles(&self) -> Result<()> {
        let selected = self.get_selected_home_dotfiles();
        if selected.is_empty() {
            return Ok(());
        }

        let user_home = std::env::var("SUDO_USER")
            .ok()
            .and_then(|u| dirs::home_dir().map(|h| h.parent().unwrap().join(&u)))
            .or_else(dirs::home_dir)
            .context("Could not determine home directory")?;

        // Copy to omarchy's default directory
        let target_dir = self.work_dir.join("configs/airootfs/root/omarchy/default/home-dotfiles");
        fs::create_dir_all(&target_dir).await?;

        for item in selected {
            // Remove trailing slash for folders
            let item_name = item.trim_end_matches('/');
            let src_path = user_home.join(item_name);
            let dest_path = target_dir.join(item_name);

            if !src_path.exists() {
                continue;
            }

            // Check if it's a directory or file
            let metadata = fs::metadata(&src_path).await?;
            if metadata.is_dir() {
                crate::utils::copy_dir_recursive(&src_path, &dest_path).await?;
            } else {
                fs::copy(&src_path, &dest_path).await?;
            }
        }

        Ok(())
    }

    async fn move_iso_to_parent(&mut self) -> Result<()> {
        let release_dir = self.work_dir.join("release");
        let mut entries = fs::read_dir(&release_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("iso") {
                let parent_dir = self.work_dir.parent().unwrap();
                let iso_name = path.file_name().unwrap();
                let final_path = parent_dir.join(iso_name);

                fs::rename(&path, &final_path).await?;
                self.iso_path = Some(final_path.to_string_lossy().to_string());

                // Clean up build directory
                fs::remove_dir_all(&self.work_dir).await?;

                return Ok(());
            }
        }

        anyhow::bail!("ISO file not found")
    }
}
