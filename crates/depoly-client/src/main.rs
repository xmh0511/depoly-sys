use std::{path::Path};

use clap::Parser;
use reqwest::blocking::multipart;
use serde_json::{Value};

#[derive(Parser, Debug)]
struct Args{
	#[arg(short, long)]
	remote:String,
	#[arg(short, long)]
	token:String,
	#[arg(short, long, default_value_t = String::from("http"))]
	protocol:String
}
fn main() ->anyhow::Result<()> {
	let args = Args::parse();
	let current_dir = std::env::current_dir()?;
	//let current_dir = Path::new("/Users/xieminghao/Documents/rust-workspace/test");
	let temp_dir = std::env::temp_dir();
	let file_name = format!("{}.zip",uuid::Uuid::new_v4().to_string());
	let temp_zip = temp_dir.join(Path::new(&file_name));
	let mut bar = progress::Bar::new();
	bar.set_job_title("Package Objects");
	file_core::compress_to_zip(current_dir.to_str().ok_or(anyhow::anyhow!("invalid path {:?}",current_dir))?, temp_zip.to_str().ok_or(anyhow::anyhow!("invalid temporary path {:?}",temp_zip))?, Some(move |msg:file_core::Message|{
		bar.reach_percent((msg.progress * 100 as f64) as i32);
		//std::thread::sleep(std::time::Duration::from_secs(1));
	}))?;
	println!();

    let token = args.token;
	let remote_ip = args.remote;
	let protocol = args.protocol;
	let remote = format!("{protocol}://{remote_ip}");//args.remote;
	let file_size = temp_zip.metadata()?.len().to_string();
	//println!("file size: {}, path: {}",file_size,temp_zip.display());
	let form_data = multipart::Form::new().text("token".to_owned(), token).text("file_size".to_owned(), file_size).file("file", temp_zip)?;
	let client = reqwest::blocking::Client::new();
	println!("Starting to upload objects to the remote server");
	let resp = client.post(format!("{remote}/depoly")).multipart(form_data).send()?;
	let r = resp.json::<Value>()?;
	if r.get("status").ok_or(anyhow::anyhow!("key \"status\" not exist in response body"))?.as_u64().unwrap_or(0) == 200{
		println!("Depoly successfully");
	}else{
		println!("Error {:?}",r);
	}
	Ok(())
}
