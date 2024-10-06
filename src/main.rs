use core::str;
use std::collections::HashMap;
use std::collections::HashSet;
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

struct ParsingResult {
	comm_id: String,
	title_id: String,
	title: String,
}

fn process_tropconf(path: &Path) -> Result<String, ErrorType> {
	let path_conf = path.with_file_name("TROPCONF.SFM");

	let mut file = fs::File::open(path_conf).map_err(ErrorType::FileIO)?;
	let mut data = Vec::new();
	file.read_to_end(&mut data).map_err(ErrorType::FileIO)?;

	let str = str::from_utf8(&data[0x40..]).unwrap();
	let (_, data) = str.split_once("<title-name>").ok_or_else(|| ErrorType::MissingData)?;
	let (name, _) = data.split_once("</title-name>").ok_or_else(|| ErrorType::MissingData)?;

	Ok(String::from(name))
}

fn process_file(path: &Path) -> Result<ParsingResult, ErrorType> {
	if path.file_name().unwrap().to_str().unwrap() != "PARAM.SFO" {
		return Err(ErrorType::FileNotParam);
	}

	let mut file = fs::File::open(path).map_err(ErrorType::FileIO)?;
	let mut data = Vec::new();
	file.read_to_end(&mut data).map_err(ErrorType::FileIO)?;

	let (_, parsed_data) = parser::parse_param_sfo(&data).map_err(|_| ErrorType::ParsingError)?;

	if !parsed_data.contains_key("NPCOMMID") || !parsed_data.contains_key("TITLEID000") {
		return Err(ErrorType::MissingData);
	}

	let str_commid = parsed_data["NPCOMMID"].to_string();
	let comm_id = if let Some((id, _)) = str_commid.split_once('_') {
		id.to_owned()
	} else {
		return Err(ErrorType::MissingData);
	};

	let title_id = parsed_data["TITLEID000"].to_string();

	let title = match process_tropconf(path) {
		Ok(v) => v,
		_ => String::new(),
	};

	Ok(ParsingResult { comm_id, title_id, title })
}

struct CommIdInfo {
	titles: HashSet<String>,
	title_ids: HashSet<String>,
}

impl CommIdInfo {
	fn default() -> CommIdInfo {
		CommIdInfo {
			titles: HashSet::new(),
			title_ids: HashSet::new(),
		}
	}
}

fn process_directory(dir: &Path, results: &mut HashMap<String, CommIdInfo>) -> anyhow::Result<()> {
	let dir = fs::read_dir(dir)?;

	for d in dir {
		let d = d?;
		let path = d.path();
		if path.is_dir() {
			process_directory(&path, results)?;
		} else {
			let res = process_file(&path);
			if res.is_err() {
				continue;
			}
			let res = res.unwrap();

			let entry = results.entry(res.comm_id).or_insert(CommIdInfo::default());
			entry.title_ids.insert(res.title_id);

			if !res.title.is_empty() {
				entry.titles.insert(res.title);
			}
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

	let mut results: HashMap<String, CommIdInfo> = HashMap::new();

	if let Err(e) = process_directory(Path::new(&args[1]), &mut results) {
		println!("Failed: {}", e);
	}

	let mut json_data = json::JsonValue::new_object();

	for (com_id, data) in &results {
		let mut array = json::JsonValue::new_array();
		for title_id in &data.title_ids {
			array.push(json::JsonValue::String(title_id.clone())).unwrap();
		}
		json_data[com_id]["title_ids"] = array;

		let mut array = json::JsonValue::new_array();
		for title in &data.titles {
			array.push(title.clone()).unwrap();
		}
		json_data[com_id]["titles"] = array;
	}

	println!("{}", json::stringify_pretty(json_data, 8));
}
