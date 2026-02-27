namespace composesiren.dsp.processor

open composesiren.dsp
open composesiren.dsp.ComposeSirenesLouetteDsp

open composesiren.dsp.state

//type SirenDspStqIndex

type ProcessorState = {

    sirens : SirenOrchestraDspState

}

type JuceMidiBuffer = | JuceMidiBuffer

type UIEvents =
    | AdjustSirenControl of parameterooo: SirenDspParameterId * value: uint8
    | MidiEvents of JuceMidiBuffer
    | ReplyStateQuery of toCallback : (ProcessorState -> unit)

