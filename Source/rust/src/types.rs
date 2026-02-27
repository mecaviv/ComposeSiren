
#[repr(C)]
pub enum SirenModel {
  Alto1,
  Alto2,
  Bass,
  Tenor,
  Soprano,
  Piccolo,
}

#[derive(PartialEq,Eq,Hash)]
#[repr(C)]
pub enum SirenDataFileSuffix {
  S1,
  S2,
  S3,
  S4,
  S5,
  //S6,
  S7,
}

pub const NOMBRE_DE_NOTE: usize = 80;
pub const MAX_Partiel: usize = 200;
pub const MAX_TAB: usize = 1000;

#[repr(C)]
pub struct Array3D<T, const Dim1: usize, const Dim2: usize, const Dim3: usize>
{
  pub data: ndarray::Array3<T>

}

impl<T, const Dim1: usize, const Dim2: usize, const Dim3: usize> Array3D<T, Dim1, Dim2, Dim3> {
  pub fn dim_1 () -> usize { Dim1 }
  pub fn dim_2 () -> usize { Dim2 }
  pub fn dim_3 () -> usize { Dim3 }
  pub fn size_of() -> usize {
    std::mem::size_of::<T>() * Dim1 * Dim2 * Dim3
  }
}

#[repr(C)]
pub struct Array1D<T, const Dim1: usize>
{
  pub data: ndarray::Array1<T>
}

impl<T, const Dim1: usize> Array1D<T, Dim1>{
  pub fn dim_1 () -> usize { Dim1 }
  /*fn size_of() -> usize {
    std::mem::size_of::<T>() * Dim1
  }*/
}

#[repr(C)]
pub struct LengthTableEntry {
  pub sample_count: u32
  , pub max_tab_count: u16
  , pub average_frequency: f32
}


pub type AmplitudeTable = Array3D<f32, NOMBRE_DE_NOTE, MAX_TAB, MAX_Partiel>;

pub type FrequencyTable = Array3D<f32, NOMBRE_DE_NOTE, MAX_TAB, MAX_Partiel>;

pub type LengthTable = Array1D<LengthTableEntry, NOMBRE_DE_NOTE>;

pub type VectorTable = Array1D<f32, 392>;

//typedef float AmpTable[NOMBRE_DE_NOTE][MAX_TAB][MAX_Partiel];
//typedef float FreqTable[NOMBRE_DE_NOTE][MAX_TAB][MAX_Partiel];
//typedef float DurationTable[NOMBRE_DE_NOTE][3];
//typedef float VectorTable [392];

type i7 = i8;
pub type midi_note = i7;

#[repr(C)]
pub struct SirenDataSet {
  pub amp:    AmplitudeTable,
  pub freq:   FrequencyTable,
  pub length: LengthTable,
  pub vector: VectorTable,
}

#[repr(C)]
pub struct SirenSpec {
  lowest_note:  midi_note,
  highest_note: midi_note,
}
#[repr(C)]
pub struct SirenModelFileSet {
  pub(crate) amp:    SirenDataFileSuffix,
  pub(crate) freq:   SirenDataFileSuffix,
  pub(crate) length: SirenDataFileSuffix,
  pub(crate) vector: SirenDataFileSuffix,
}

impl std::fmt::Display for SirenDataFileSuffix {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use SirenDataFileSuffix::*;
    let result = match self {
      S1 => "S1",
      S2 => "S2",
      S3 => "S3",
      S4 => "S4",
      S5 => "S5",
      S7 => "S7",
    };
    write!(f, "{}", result)
  }
}
impl std::fmt::Display for SirenModelFileSet {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f,
           "SirenDataSet(freq:{},amp:{},length:{},vector:{})",
           self.freq, self.amp, self.length, self.vector)
  }
}