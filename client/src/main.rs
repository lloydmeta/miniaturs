use std::{env, error::Error};

use anyhow::Context;
use miniaturs_shared::signature::*;

const SHARED_SECRET_ENV_KEY: &'static str = "MINIATURS_SHARED_SECRET";
fn main() -> Result<(), Box<dyn Error>> {
    let shared_secret = env::var(SHARED_SECRET_ENV_KEY)
        .context("Expected {SHARED_SECRET_ENV_KEY} to be defined")?;

    let args: Vec<_> = env::args().collect();

    let to_sign = &args.get(1).context("Expected an argument to sign")?;
    let signed = make_url_safe_base64_hash(&shared_secret, to_sign).context("Failed to sign")?;
    println!("{signed}/{to_sign}");

    Ok(())
}
