use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    panopticon::repl::run().await
}
