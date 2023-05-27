#r "bin/Debug/net7.0/ComposeSiren.dll"
open composesiren.dsp.processor
open composesiren.dsp.state

[<EntryPoint>]
let main args =
    let sirens =
        [|
            composesiren.declarations.Alto1    
            composesiren.declarations.Alto2    
            composesiren.declarations.Bass    
            composesiren.declarations.Tenor    
            composesiren.declarations.Soprano    
            composesiren.declarations.Soprano  
            composesiren.declarations.Piccolo  
        |]
        
    let engineState =
        {
              lastPlayedNote = None// PitchEvent option
              currentSpeed = EngineSpeed()
        }

    let shutterState = { lastShutterEvent = None }
    let sirenState =
        {
                pan = 0.
                engineState = engineState
                shutterState = shutterState
         }
        
    let orchestra : SirenOrchestraDspState =
        {
          sirens =
             [|
             
                sirenState
                sirenState
                sirenState
                sirenState
                sirenState
                sirenState
                sirenState
            |]
        }
    let state =
        {
            ProcessorState.sirens = orchestra
        }

            
        

    1