use std::collections::HashMap;

use nom::error::ErrorKind;
use nom::{
	bytes::complete::{tag, take, take_until},
	error_position,
	number::complete::*,
	Err, IResult,
};

pub enum ParamData {
	String(String),
	U32(u32),
}

struct ParamSfoHeader {
	_version: u32,
	key_table_offset: u32,
	data_table_offset: u32,
	num_entries: u32,
}

enum ParamSfoDataType {
	Utf8NoNull,
	Utf8,
	U32,
}

struct ParamSfoEntry {
	key_offset: u16,
	fmt: ParamSfoDataType,
	len: u32,
	max_len: u32,
	data_offset: u32,
}

type ParamSfoData = HashMap<String, ParamData>;

impl std::fmt::Display for ParamData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ParamData::String(s) => write!(f, "{}", s),
			ParamData::U32(num) => write!(f, "{}", num),
		}
	}
}

fn parse_header(input: &[u8]) -> IResult<&[u8], ParamSfoHeader> {
	let (input, _) = tag(b"\x00PSF")(input)?;
	let (input, version) = le_u32(input)?;
	let (input, key_table_offset) = le_u32(input)?;
	let (input, data_table_offset) = le_u32(input)?;
	let (input, num_entries) = le_u32(input)?;

	Ok((
		input,
		ParamSfoHeader {
			_version: version,
			key_table_offset,
			data_table_offset,
			num_entries,
		},
	))
}

fn parse_entry(input: &[u8]) -> IResult<&[u8], ParamSfoEntry> {
	let (input, key_offset) = le_u16(input)?;
	let (input, _) = tag([4u8])(input)?;
	let (input, fmt) = le_u8(input)?;

	let fmt = match fmt {
		0 => ParamSfoDataType::Utf8NoNull,
		2 => ParamSfoDataType::Utf8,
		4 => ParamSfoDataType::U32,
		_ => return Err(Err::Error(error_position!(input, ErrorKind::IsNot))),
	};

	let (input, len) = le_u32(input)?;
	let (input, max_len) = le_u32(input)?;
	let (input, data_offset) = le_u32(input)?;

	Ok((
		input,
		ParamSfoEntry {
			key_offset,
			fmt,
			len,
			max_len,
			data_offset,
		},
	))
}

fn parse_data<'a>(input: &'a [u8], total_input: &'a [u8], header: &ParamSfoHeader) -> IResult<&'a [u8], ParamSfoData> {
	let mut entry_input = input;

	let mut final_data = HashMap::new();

	for _ in 0..header.num_entries {
		let (input, entry) = parse_entry(entry_input)?;
		entry_input = input;

		let key_offset = entry.key_offset as usize + header.key_table_offset as usize;
		let data_offset = entry.data_offset as usize + header.data_table_offset as usize;

		if total_input.len() < key_offset + 1 || total_input.len() < data_offset + entry.max_len as usize {
			return Err(Err::Error(error_position!(input, ErrorKind::Eof)));
		}

		let (_, key) = take_until("\0")(&total_input[key_offset..])?;
		let key = std::str::from_utf8(key).map_err(|_| Err::Error(error_position!(input, ErrorKind::Satisfy)))?.to_owned();

		let data = match entry.fmt {
			ParamSfoDataType::Utf8NoNull => {
				let (_, entry_data) = take(entry.len)(&total_input[data_offset..])?;
				let data_string = std::str::from_utf8(entry_data).map_err(|_| Err::Error(error_position!(input, ErrorKind::Satisfy)))?.to_owned();
				ParamData::String(data_string)
			}
			ParamSfoDataType::Utf8 => {
				let (_, entry_data) = take_until("\0")(&total_input[data_offset..])?;
				let data_string = std::str::from_utf8(entry_data).map_err(|_| Err::Error(error_position!(input, ErrorKind::Satisfy)))?.to_owned();
				ParamData::String(data_string)
			}
			ParamSfoDataType::U32 => {
				let (_, num) = le_u32(&total_input[data_offset..])?;
				ParamData::U32(num)
			}
		};

		final_data.insert(key, data);
	}

	Ok((input, final_data))
}

pub fn parse_param_sfo(total_input: &[u8]) -> IResult<&[u8], ParamSfoData> {
	let (input, header) = parse_header(total_input)?;
	let (_, data) = parse_data(input, total_input, &header)?;

	Ok((input, data))
}
