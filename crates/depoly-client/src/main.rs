use std::path::Path;

use clap::Parser;
use reqwest::{blocking::multipart, StatusCode};
use serde_json::Value;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    remote: String,
    #[arg(short, long)]
    token: String,
    // #[arg(short, long, default_value_t = String::from("http"))]
    // protocol: String,
    #[arg(long, default_value_t = 3)]
    retry: i32,
}
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let current_dir = std::env::current_dir()?;
    let retry_count = args.retry;
    'Complete: loop {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("{}.zip", uuid::Uuid::new_v4().to_string());
        let temp_zip = temp_dir.join(Path::new(&file_name));
        //println!("save path {}",temp_zip.to_string_lossy());
        //let mut bar = progress::Bar::new();
        let line_progress = indicatif::ProgressBar::new(100);
        // bar.set_job_title("Package Objects");
        file_core::compress_to_zip(
            current_dir
                .to_str()
                .ok_or(anyhow::anyhow!("invalid path {:?}", current_dir))?,
            temp_zip
                .to_str()
                .ok_or(anyhow::anyhow!("invalid temporary path {:?}", temp_zip))?,
            Some(|msg: file_core::Message| {
                line_progress.set_position((msg.progress * 100f64) as u64);
            }),
        )?;
        line_progress.finish_with_message("Package objects Done!!!");

        let token = args.token.clone();
        // let remote_ip = args.remote.clone();
        // let protocol = args.protocol.clone();
        let remote = args.remote.clone(); //format!("{protocol}://{remote_ip}"); //args.remote;
        let file_size = temp_zip.metadata()?.len().to_string();
        let client = reqwest::blocking::Client::new();
        println!("Start to upload objects to the remote server");

        for _ in 0..retry_count {
            let form_data = multipart::Form::new()
                .text("token".to_owned(), token.clone())
                .text("file_size".to_owned(), file_size.clone())
                .file("file", temp_zip.clone())?;
            let resp = client
                .post(format!("{remote}/depoly"))
                .multipart(form_data)
                .send()?;
            if resp.status() == StatusCode::OK {
                let r = resp.json::<Value>()?;
                let code = r
                    .get("status")
                    .ok_or(anyhow::anyhow!("key \"status\" not exist in response body"))?
                    .as_u64()
                    .unwrap_or(0);
                if code == 200 {
                    println!("Deploy successfully");
                    std::fs::remove_file(temp_zip)?;
                    break 'Complete;
                } else if code == 100 {
                    //upload size issue
                    println!("spurious error, retrying");
                    continue;
                } else if code == 101 {
                    //zip archive issue
                    println!(
                        "Error:\n {}\n, retrying all",
                        serde_json::to_string_pretty(&r)?
                    );
                    std::fs::remove_file(temp_zip)?;
                    continue 'Complete;
                } else {
                    println!("Error:\n {}", serde_json::to_string_pretty(&r)?);
                    std::fs::remove_file(temp_zip)?;
                    break 'Complete;
                }
            } else {
                let r = resp.json::<Value>()?;
                println!("Error:\n {}", serde_json::to_string_pretty(&r)?);
                std::fs::remove_file(temp_zip)?;
                break 'Complete;
            }
        }
    }
    Ok(())
}
