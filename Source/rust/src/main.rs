extern crate mecaviv_siren_declarations_rs;

//use src;
///++.use crate::meaviv_siren_declarations_rs::composesiren::dsp;
//use crate::composesiren::dsp;

use std::collections::HashMap;
//use composesiren::dsp::SirenModel;
//use mecaviv_siren_declarations_rs::{*};
//use mecaviv_siren_declarations_rs::composesiren::dsp::processor::{*};
fn main() {

    let sirens = Default::default();
    let state = mecaviv_siren_declarations_rs::composesiren::DspState::ProcessorState{ sirens: sirens };

    println!("Hello, world!");
    //let map = HashMap::new();
    let folder = "/Users/gauthiersegay/m1/macbookairm1/Library/Developer/Xcode/DerivedData/ComposeSirenes-docslgnkxmlfdudsoqsmkhkympli/Build/Products/Debug/ComposeSirenes.app/Contents/Resources/";
//composesiren_dsp_rs::types::SirenModel
    //read_siren_dataset(&SirenModel::Alto1, folder, &map);
    //libcomposesirendataset::pppp(SirenModel::Alto1, &map);
}