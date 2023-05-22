namespace composesiren.dsp

open composesiren.declarations
//open System.IO
open System.Collections.Generic

(*
type SirenId =
    | Piccolo
    | AltoS1
    | AltoS2
    | Bass| Tenor |SopranoS5 | SopranoS6
*)

type SirenModelFileSet =
    {
      amp : SirenDataFileSuffix
      freq: SirenDataFileSuffix
      length: SirenDataFileSuffix
      vector: SirenDataFileSuffix
    }

type FileType =
    | Amp
    | Freq
    | Length
    | Vector // deprecated by approximation formula

module ComposeSirenesLouetteDsp =

    type ReverbParameterId =
        | Size
        | DryWet
        | Depth
        | Width

    type SirenDspParameterId =
        //| DspStereoPan of siren: SirenId
        | DspGain
        | DspPartialCount
        | ReverParameter of reverParameter: ReverbParameterId


    type LengthTable =
        {
          sample_count: uint32
          max_tab_count: uint16
          average_frequency: float32
        }

    let NOMBRE_DE_NOTE = 80

    let MAX_Partiel = 200

    let MAX_TAB = 1000




module DspState =
  [<RequireQualifiedAccess>]
  type midi_note_index = midi_note_index of uint8
  [<RequireQualifiedAccess>]
  type midi_velocity = midi_velocity of uint8

  [<RequireQualifiedAccess>]
  type midi_pitchbend = midi_pitchbend of uint16
  [<RequireQualifiedAccess>]
  type dsp_frame = dsp_frame of uint64
  type VibratoEvent =
  | NewVibratoValueSample of frame: dsp_frame * vibratoDepthValue: float
  type TremoloEvent =
  | NewTremoloValueSample of frame: dsp_frame * tremoloDepthValue: float

  type EngineSpeedUpdate =
  | LiveMidiNoteOn of note: midi_note_index
  | LiveMidiNoteOff of note: midi_note_index
  | PitchbendValue of bend: midi_pitchbend
  | VibratoEvent of VibratoEvent

  type ShutterUpdateEvent =
    | TremoloEvent of TremoloEvent
    | LiveMidiNoteOn of note: midi_note_index
    | LiveMidiNoteOff of note: midi_note_index
  type ShutterState =
    {
      lastShutterEvent: ShutterUpdateEvent option
    }
  //| TuningMap // todo v3
  type PitchEvent =
  | MidiNote of midi_note_index * midi_velocity // add isOn isOff, for now, if velocity is 0, it is note off

  type EngineSpeed() =
    class
    end

  type EngineState =
    {
      lastPlayedNote: PitchEvent option
      currentSpeed: EngineSpeed


    }

  type SirenVoiceDspState =
      {
        pan: float
        engineState: EngineState
        shutterState: ShutterState
      }
  type SirenDspState =
      {
          sirens : SirenVoiceDspState array

      }

#if false
module Routines =
    open ComposeSirenesLouetteDsp

    let sirenmodel_get_file_set (sirenModel: SirenModel) : SirenModelFileSet =
      let amp, length, vector, freq =
        match sirenModel with
        |Alto1 -> (S1, S1, S1, S1)
        |Alto2 -> (S1, S1, S2, S1)
        |Bass -> (S3, S3, S3, S3)
        |Tenor -> (S4, S4, S4, S4)
        |Soprano -> (S5, S5, S5, S5)
        |Piccolo -> (S7, S7, S5, S7)
      { amp=amp
        freq=freq
        length=length
        vector=vector}

    let get_filename_and_buffer_size file (suffix: SirenDataFileSuffix) (directory:DirectoryInfo) =
      //let s = sizeof<AmplitudeTable>
      let filePath, bufferSize =
        let amplitudeTable = NOMBRE_DE_NOTE * MAX_TAB * MAX_Partiel
        let lengthTable =  NOMBRE_DE_NOTE * (*uint32 * uint16 * f32*) 4 * 2 * 4  //sizeof<LengthTable> //"" // LengthTable::dim_1() * std::mem::size_of::<f32>() * 3)
        let dataVectorInterval = (*f32*) 4 * 392
        let frequencyTable =  NOMBRE_DE_NOTE * MAX_TAB * MAX_Partiel
        match file with
        | FileType.Amp -> "dataAmp", amplitudeTable
        | FileType.Freq -> "dataFreq", frequencyTable //FrequencyTable::size_of()
        | FileType.Length -> "datadureTabs", lengthTable
        | FileType.Vector -> "dataVectorInterval", dataVectorInterval //, VectorTable::dim_1() * std::mem::size_of::<f32>())

      let isEqual = 0 = bufferSize % 4
      if not isEqual then
        failwith "rustcodetodo: assert_eq!(0, bufferSize % 4)"

      let directoryName =  $"filename{file}"// directory.Name
      let filePath = directoryName + "/" + filePath + string suffix
      filePath, bufferSize

    //type FileMap = System.Collections.Generic.IDictionary<Box<str>,Box<Vec<u8>>>
    type FileMap = System.Collections.Generic.IDictionary<string,ResizeArray<byte>>


    let read_siren_dataset (sirenModel: SirenModel) (folder: DirectoryInfo) (fileMap: FileMap) =

      let fileSet = sirenmodel_get_file_set(sirenModel);

      let readFileAsFloatsVector (f: FileType) (suffix: SirenDataFileSuffix) =
        let file_path, buffer_size = get_filename_and_buffer_size(f, suffix, folder)
        let fileInfo = FileInfo filePath
        let buffer_size = buffer_size / sizeof<float32> // std::mem::size_of::<f32>();
        let file_size = fileInfo.Length //std::fs::metadata(&file_path).expect("could not get file meta data").len() as usize;
        let datafile =
            //
            failwith $"rustcodetodo: std::fs::File::open(&file_path).expect(&format!(  not able to read {fileInfo.FullName})"
        //let datafile = BufReader::new(datafile);
        (*let mut floats_buffer = {
          let mut floats = Vec::<f32>::with_capacity(buffer_size);
          unsafe { floats.set_len(file_size / std::mem::size_of::<f32>()); }

          datafile.read_f32_into::<NativeEndian>(&mut floats[..]).unwrap();
          floats
        };
        if buffer_size != floats_buffer.len() {
          eprintln!("expected buffersize:{} got:{} {}", buffer_size, floats_buffer.len(), file_path.as_str());
          floats_buffer.resize(buffer_size, 0.);
        }
        *)
        let float_buffer = ()
        floats_buffer

          (*
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
            *)
      let sirenDataset =
         {
             amp = amp
             freq= freq
             length = length
             vector = vector
           }
      sirenDataset
#endif

//module TestRoutines =
//#[cfg(test)]

  //use std::collections::HashMap;
  //use crate::{sirenmodel_get_file_set, read_siren_dataset};

  //use crate::types::SirenModel::{*};
  //const folder : &str = "/Users/gauthiersegay/dev/src/mecaviv/benoit/CodesSources/ComposeSirenes2/ComposeSirenes/";

  //[<Test>]
#if false
  let it_worksreadfiles() =
    let map = Dictionary()
    let s1 = read_siren_dataset(&crate::types::SirenModel::Alto2, folder, &map);
    for v in s1.vector do
      //let v = s1.vector.data[i];
      printfn $"{v}"



  //[<Test>]
  let it_works () =
    let s1 = sirenmodel_get_file_set(&Alto1)
    let s2 = sirenmodel_get_file_set(&Alto2)
    let s3 = sirenmodel_get_file_set(&Bass)
    let s4 = sirenmodel_get_file_set(&Tenor)
    let s5 = sirenmodel_get_file_set(&Soprano)
    let s6 = sirenmodel_get_file_set(&Soprano)
    let s7 = sirenmodel_get_file_set(&Piccolo)

    printfn $"s1={s1}"
    printfn $"s2={s2}"
    printfn $"s3={s3}"
    printfn $"s4={s4}"
    printfn $"s5={s5}"
    printfn $"s6={s6}"
    printfn $"s7={s7}"
    //assert_eq!(2 + 2, 4);


#endif