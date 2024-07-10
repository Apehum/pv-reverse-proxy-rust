#![feature(cursor_remaining)]

use std::io::Error;
use crate::proxy::VoiceProxy;

mod proxy;
mod packet;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut proxy = VoiceProxy::new(30000).await?;
    
    proxy.listen().await?;
    
    Ok(())
}
