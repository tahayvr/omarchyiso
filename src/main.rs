mod app;
mod config;
mod ui;
mod utils;

use anyhow::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new()?;
    app.run().await?;
    Ok(())
}
