namespace composesiren
open Fable.Core.Rust
[<Struct;OuterAttr("repr(C)")>]
type SirenLibrary = 
    {
        version: string
    }

module refs_ =
  importAll "composesiren.declarations.rs"
  importAll "composesiren.dsp.rs"
  importAll "composesiren.dsp.processor.rs"