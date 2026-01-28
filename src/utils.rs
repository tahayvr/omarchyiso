use anyhow::Result;
use futures::future::BoxFuture;
use std::path::Path;
use tokio::fs;

pub fn copy_dir_recursive<'a>(src: &'a Path, dst: &'a Path) -> BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        fs::create_dir_all(dst).await?;

        let mut entries = fs::read_dir(src).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                copy_dir_recursive(&src_path, &dst_path).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    })
}
