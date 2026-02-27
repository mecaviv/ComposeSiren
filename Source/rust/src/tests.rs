
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
