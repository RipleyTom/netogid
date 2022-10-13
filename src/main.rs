use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;

use thiserror::Error;

mod parser;

#[derive(Error, Debug)]
enum ErrorType {
	#[error("IO Error")]
	FileIO(std::io::Error),
	#[error("File is not a PARAM.SFO file")]
	FileNotParam,
	#[error("Parsing error")]
	ParsingError,
	#[error("Missing data")]
	MissingData,
}

fn process_file(file: &Path) -> Result<String, ErrorType> {
	if file.file_name().unwrap().to_str().unwrap() != "PARAM.SFO" {
		return Err(ErrorType::FileNotParam);
	}

	let mut file = fs::File::open(file).map_err(ErrorType::FileIO)?;
	let mut data = Vec::new();

	file.read_to_end(&mut data).map_err(ErrorType::FileIO)?;

	let (_, parsed_data) = parser::parse_param_sfo(&data).map_err(|_| ErrorType::ParsingError)?;

	if !parsed_data.contains_key("NPCOMMID") || !parsed_data.contains_key("TITLEID000") {
		return Err(ErrorType::MissingData);
	}

	let str_commid = parsed_data["NPCOMMID"].to_string();
	let parsed_comm_id = if let Some((id, _)) = str_commid.split_once('_') {
		id.to_owned()
	} else {
		return Err(ErrorType::MissingData);
	};

	Ok(format!("{}=>{}", parsed_comm_id, parsed_data["TITLEID000"]))
}

fn process_directory(dir: &Path) -> anyhow::Result<()> {
	let dir = fs::read_dir(dir)?;

	for d in dir {
		let d = d?;
		let path = d.path();
		if path.is_dir() {
			process_directory(&path)?;
		} else {
			let res = process_file(&path);
			if res.is_err() {
				continue;
			}
			let res = res.unwrap();
			println!("{}", res);
		}
	}

	Ok(())
}

fn main() {
	println!("Network ID to Game ID v{}", env!("CARGO_PKG_VERSION"));

	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("Syntax:");
		println!("{} <directory>", args[0]);
		return;
	}

	if let Err(e) = process_directory(Path::new(&args[1])) {
		println!("Failed: {}", e);
	}
}
