
use zip::write::FileOptions;
use std::{path::{PathBuf,Path}};
use ignore::Walk;
use std::io::{Write,Read};
pub struct Message{
	pub progress:f64,
	pub current_document:String,
	pub document_size:u64
}
fn scan_dir(src:&str)->anyhow::Result<Vec<PathBuf>>{
	let mut files = Vec::new();
	let root_path = Path::new(src);
	for result in Walk::new(root_path) {
		match result {
			Ok(entry) => {
				files.push(entry.path().to_owned());
			},
			Err(err) => {
				return Err(anyhow::anyhow!("ERROR: {}", err));
			},
		}
	}
	Ok(files)
}
pub fn compress_to_zip<F:FnMut(Message)>(src:&str,dst:&str,mut call_back:Option<F>)->anyhow::Result<()>{
	let writer = std::fs::File::create(dst)?;
	let mut zip = zip::ZipWriter::new(writer);
	let options = FileOptions::default()
	.compression_method(zip::CompressionMethod::Deflated)
	.unix_permissions(0o755);
    let content_path:Vec<PathBuf> = scan_dir(src)?;
    let mut buffer = Vec::new();
	let mut index = 0usize;
	let total_tasks = content_path.len();
    for it in &content_path{
		let path = it.as_path();
		let name = path.strip_prefix(Path::new(src))?.to_str().ok_or(anyhow::anyhow!("invalid file name"))?;
		if it.is_file(){
			//println!("adding file {path:?} as {name:?} ...");
			zip.start_file(name, options)?;
			let mut f = std::fs::File::open(path)?;
			f.read_to_end(&mut buffer)?;
			zip.write_all(&buffer)?;
			buffer.clear();
		}else{
			//println!("adding dir {path:?} as {name:?} ...");
			zip.add_directory(name, options)?;
		}
		index+=1;
		if let Some(ref mut f) = call_back{
			let msg = Message{
				progress:index as f64 / total_tasks as f64,
				current_document:name.to_owned(),
				document_size: match it.metadata(){
					Ok(d)=>{d.len()}
					Err(_)=>{0}
				}
			};
			f(msg);
		};
	}
	zip.finish()?;
	Ok(())
}

pub fn decompress_zip_to_dir<F:FnMut(Message)>(src:&str,dest:&str, mut call_back:Option<F>)->anyhow::Result<Option<Vec<String>>>{
	let file = std::fs::File::open(src)?;
	let mut archive = zip::ZipArchive::new(file)?;
	let dest_root_path = Path::new(dest);
	if !dest_root_path.exists(){
		std::fs::create_dir(dest_root_path)?;
	}
	let mut index = 0usize;
	let total_tasks = archive.len();
	let mut fail_files = Vec::new();
	for i in 0..archive.len(){
		let mut file = archive.by_index(i)?;
		let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None =>{
				index+=1;
				if let Some(ref mut f) = call_back{
					let msg = Message{
						progress:index as f64 / total_tasks as f64,
						current_document:String::from(""),
						document_size: 0
					};
					f(msg);
				};
				continue;
			},
        };
        let prefix_path = Path::new(dest);
		let relative_path = outpath.clone();
		let outpath = prefix_path.join(&outpath);
		if (*file.name()).ends_with('/') {
            //println!("File {} extracted to \"{}\"", i, outpath.display());
            std::fs::create_dir_all(&outpath)?;
        } else {
            // println!(
            //     "File {} extracted to \"{}\" ({} bytes)",
            //     i,
            //     outpath.display(),
            //     file.size()
            // );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
			println!("process {}",relative_path.display());
            match std::io::copy(&mut file, &mut outfile){
                Ok(_) => {},
                Err(e) => {
					let err = format!("fail to place {}, reson:{e:?}",relative_path.display());
					fail_files.push(err);
				},
            }
        }
		// Get and Set permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			if let Some(mode) = file.unix_mode() {
				match std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)){
					Ok(_)=>{},
					Err(e)=>{
						let err = format!("fail to give permission to {}, reson:{e:?}",relative_path.display());
						fail_files.push(err);
					}
				};
			}
		}
		index+=1;
		if let Some(ref mut f) = call_back{
			let msg = Message{
				progress:index as f64 / total_tasks as f64,
				current_document:relative_path.to_str().unwrap_or("").to_owned(),
				document_size: file.size()
			};
			f(msg);
		};
	}
	if fail_files.is_empty(){
		Ok(None)
	}else{
		Ok(Some(fail_files))
	}
}