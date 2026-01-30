mod app;
mod config;
mod ui;
mod utils;

use anyhow::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut dev_mode = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--dev" => dev_mode = true,
            "-h" | "--help" => {
                println!(
                    "omarchyiso\n\nUsage:\n  omarchyiso [--dev]\n\nOptions:\n  --dev    Enable dev mode (persist build log to ./omarchyiso_logs/)\n  -h, --help  Show this help\n"
                );
                return Ok(());
            }
            other => {
                anyhow::bail!("Unknown argument: {other}\n\nTry: omarchyiso --help");
            }
        }
    }

    let mut app = App::new(dev_mode)?;
    app.run().await?;
    Ok(())
}
