namespace composesiren.declarations
open Fable.Core.Rust
[<Struct;OuterAttr("repr(C)")>]
type SirenDataFileSuffix =
| S1
| S2
| S3
| S4
| S5
| S7

[<Struct;OuterAttr("repr(C)")>]
type SirenModel =
| Alto1
| Alto2
| Bass
| Tenor
| Soprano
| Piccolo
