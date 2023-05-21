//use composesiren;
//use composesiren::dsp;
//use composesiren::declarations;
//#![feature(type_ascription)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(unused_variables)]
//pub mod midiinput;
pub mod types;
//pub mod ndarraytests;
//pub mod siren_send;
//pub const midiNoteMin: i32 = 0;
//pub const midiNoteMax: crate::types::midi_note = 127;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{BufReader, Read};
use std::iter::Map;
use std::os::raw::c_char;
use std::path::Path;
use std::rc::Rc;
use byteorder::{NativeEndian, ReadBytesExt};
use crate::types::{AmplitudeTable, FrequencyTable, LengthTable, LengthTableEntry, MAX_Partiel, MAX_TAB, NOMBRE_DE_NOTE, SirenDataFileSuffix, SirenDataSet, SirenModel, SirenModelFileSet, VectorTable};

#[no_mangle]
pub extern "C" fn sirenmodel_get_file_set(sirenModel: &SirenModel) -> SirenModelFileSet {
  use crate::types::SirenModel::{*};

  use SirenDataFileSuffix::*;
  let (amp, length, vector, freq) = match sirenModel {
    Alto1 => (S1, S1, S1, S1),
    Alto2 => (S1, S1, S2, S1),
    Bass => (S3, S3, S3, S3),
    Tenor => (S4, S4, S4, S4),
    Soprano => (S5, S5, S5, S5),
    Piccolo => (S7, S7, S5, S7),
  };
  SirenModelFileSet { amp,
                      freq,
                      length,
                      vector }
}

#[derive(PartialEq,Eq,Hash)]
enum FileType {
  Amp,
  Freq,
  Length,
  Vector,
}
/*
enum FileData {
  Amp(Box<AmplitudeTable>),
  Freq(Box<FrequencyTable>),
  Length(Box<LengthTable>),
  Vector(Box<VectorTable>),
}*/

fn get_filename_and_buffer_size(f: &FileType, suffix: &SirenDataFileSuffix, folder: &str) -> (String, usize) {
  let s = AmplitudeTable::size_of();
  let (filePath, bufferSize) = match f {
    FileType::Amp => ("dataAmp", AmplitudeTable::size_of()),
    FileType::Freq => ("dataFreq", FrequencyTable::size_of()),
    FileType::Length => ("datadureTabs", LengthTable::dim_1() * std::mem::size_of::<f32>() * 3),
    FileType::Vector => ("dataVectorInterval", VectorTable::dim_1() * std::mem::size_of::<f32>()),
  };
  assert_eq!(0, bufferSize % 4);
  let filePath =
      format!("{}{}{}",
              folder,filePath, suffix);

  (filePath, bufferSize)
}

type FileMap = HashMap<Box<str>,Box<Vec<u8>>>;


use lazy_static::lazy_static;
use ndarray::{Array1, Array3};

lazy_static! {
    static ref PRIVILEGES: HashMap<&'static str, Vec<&'static u8>> = {
        let mut map = HashMap::new();
        map
    };
}


pub extern "C" fn read_siren_dataset(sirenModel: &SirenModel, folder: &str, mut fileMap: &FileMap) -> SirenDataSet //-> SirenDataSet
{
  let fileSet = sirenmodel_get_file_set(sirenModel);

  let readFileAsFloatsVector = |f: &FileType, suffix: &SirenDataFileSuffix| {
    let (file_path, buffer_size) = get_filename_and_buffer_size(f, suffix, folder);
    let buffer_size = buffer_size / std::mem::size_of::<f32>();
    let file_size = std::fs::metadata(&file_path).expect("could not get file meta data").len() as usize;
    let datafile = std::fs::File::open(&file_path).expect(&format!("not able to read {}", file_path));
    let mut datafile = BufReader::new(datafile);
    let mut floats_buffer = {
      let mut floats = Vec::<f32>::with_capacity(buffer_size);
      unsafe { floats.set_len(file_size / std::mem::size_of::<f32>()); }

      datafile.read_f32_into::<NativeEndian>(&mut floats[..]).unwrap();
      floats
    };
    if buffer_size != floats_buffer.len() {
      eprintln!("expected buffersize:{} got:{} {}", buffer_size, floats_buffer.len(), file_path.as_str());
      floats_buffer.resize(buffer_size, 0.);
    }

    floats_buffer
  };

  let amp    = readFileAsFloatsVector(&FileType::Amp, &fileSet.amp);
  let freq   = readFileAsFloatsVector(&FileType::Freq, &fileSet.freq);
  let length = readFileAsFloatsVector(&FileType::Length, &fileSet.length);
  let vector = readFileAsFloatsVector(&FileType::Vector, &fileSet.vector);

  let amp = AmplitudeTable { data: Array3::from_shape_vec((AmplitudeTable::dim_1(), AmplitudeTable::dim_2(), AmplitudeTable::dim_3()), amp).unwrap() };
  let freq = FrequencyTable { data: Array3::from_shape_vec((FrequencyTable::dim_1(), FrequencyTable::dim_2(), FrequencyTable::dim_3()), freq).unwrap() };
  let length = {
    let mut v = Vec::<LengthTableEntry>::with_capacity(LengthTable::dim_1());
    for n in 0 .. LengthTable::dim_1() {
      let (sample_count, max_tab_count, average_frequency) =
          (length[n * 3]
          , length[n * 3 + 1]
          , length[n * 3 + 2])
          ;
      let entry = LengthTableEntry {
        sample_count: sample_count.round() as u32
        , max_tab_count : max_tab_count.round() as u16
        , average_frequency
      };
      v.push(entry);
    }
    if v.len() < LengthTable::dim_1() {
      eprintln!("expected length table of length {} but got {}", LengthTable::dim_1(), v.len());
    }
    LengthTable { data: Array1::from_shape_vec(LengthTable::dim_1(), v).unwrap() }

  };
  let vector = VectorTable { data: Array1::from_shape_vec(VectorTable::dim_1(), vector).unwrap() };
  SirenDataSet{ amp, freq, length, vector }

}
#[cfg(test)]
pub mod tests {
  use std::collections::HashMap;
  use crate::{sirenmodel_get_file_set, read_siren_dataset};

  use crate::types::SirenModel::{*};
  const folder : &str = "/Users/gauthiersegay/dev/src/mecaviv/benoit/CodesSources/ComposeSirenes2/ComposeSirenes/";
  #[test]
  pub fn a() {}

  #[test]
  pub fn it_worksreadfiles() {
    let map = HashMap::new();
    let s1 = read_siren_dataset(&crate::types::SirenModel::Alto2, folder, &map);
    for i in 0 .. s1.vector.data.len() {
      let v = s1.vector.data[i];
      println!("{}", v);
    }
  }

  #[test]
  pub fn it_works() {
    let s1 = sirenmodel_get_file_set(&Alto1);
    let s2 = sirenmodel_get_file_set(&Alto2);
    let s3 = sirenmodel_get_file_set(&Bass);
    let s4 = sirenmodel_get_file_set(&Tenor);
    let s5 = sirenmodel_get_file_set(&Soprano);
    let s6 = sirenmodel_get_file_set(&Soprano);
    let s7 = sirenmodel_get_file_set(&Piccolo);

    println!("s1={}", s1);
    println!("s2={}", s2);
    println!("s3={}", s3);
    println!("s4={}", s4);
    println!("s5={}", s5);
    println!("s6={}", s6);
    println!("s7={}", s7);
    assert_eq!(2 + 2, 4);
  }
}



