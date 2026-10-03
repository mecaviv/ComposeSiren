// One-shot: force ClicVolume to 1.0 (and leave Spread/Bias at 0) via MCP.
//   dotnet fsi /tmp/clic-set-volume.fsx [value]

open System
open System.IO
open System.Net.Http
open System.Text
open System.Text.Json.Nodes

let discoveryPath =
    Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".composesiren_mcp.json")

let http = new HttpClient(Timeout = TimeSpan.FromSeconds 5.0)

let parseBody (body: string) =
    if String.IsNullOrWhiteSpace body then
        None
    else
        let json =
            body.Split '\n'
            |> Array.tryPick (fun line ->
                let t = line.TrimStart()

                if t.StartsWith "data:" then
                    Some(t.Substring(5).Trim())
                else
                    None)
            |> Option.defaultValue body

        try
            Some(JsonNode.Parse json)
        with _ ->
            printfn "unparseable response: %s" (body.Substring(0, min 240 body.Length))
            None

type Mcp(url: string) =
    let mutable nextId = 0
    let mutable session: string = null

    let nextRpcId () =
        nextId <- nextId + 1
        nextId

    let post (payload: string) (notify: bool) =
        task {
            use content = new StringContent(payload, Encoding.UTF8, "application/json")
            use req = new HttpRequestMessage(HttpMethod.Post, url)
            req.Content <- content
            req.Headers.Accept.ParseAdd "application/json"
            req.Headers.Accept.ParseAdd "text/event-stream"

            if not (isNull session) then
                req.Headers.TryAddWithoutValidation("Mcp-Session-Id", session) |> ignore

            let! resp = http.SendAsync req

            let sid =
                match resp.Headers.TryGetValues "Mcp-Session-Id" with
                | true, vals -> Seq.head vals
                | _ ->
                    match resp.Headers.TryGetValues "mcp-session-id" with
                    | true, vals -> Seq.head vals
                    | _ -> null

            if not (isNull sid) then
                session <- sid

            let! body = resp.Content.ReadAsStringAsync()

            if notify then
                return None
            else
                return parseBody body
        }

    do
        task {
            let init =
                $"""{{"jsonrpc":"2.0","id":{nextRpcId ()},"method":"initialize","params":{{"protocolVersion":"2024-11-05","capabilities":{{}},"clientInfo":{{"name":"clic-vol","version":"0"}}}}}}"""

            let! _ = post init false
            let! _ = post """{"jsonrpc":"2.0","method":"notifications/initialized"}""" true
            ()
        }
        |> Async.AwaitTask
        |> Async.RunSynchronously

    member _.CallTool(name: string, args: string) =
        let payload =
            $"""{{"jsonrpc":"2.0","id":{nextRpcId ()},"method":"tools/call","params":{{"name":"{name}","arguments":{args}}}}}"""

        post payload false

let main =
    task {
        let entries = JsonNode.Parse(File.ReadAllText discoveryPath)
        let e = entries.AsArray().[0]
        let port = e.["port"].GetValue<int>()
        let url = $"http://127.0.0.1:{port}/mcp"
        printfn "MCP %s pid=%d" url (e.["pid"].GetValue<int>())

        let value =
            match fsi.CommandLineArgs with
            | [| _; s |] -> float s
            | _ -> 1.0

        let mcp = Mcp url

        let! listed = mcp.CallTool("list_parameters", "{}")
        match listed with
        | Some node ->
            let text = node.ToJsonString()
            for line in text.Split '\n' do
                if line.Contains "Clic" || line.Contains "clic" then
                    printfn "%s" (line.Trim())
        | None ->
            printfn "list_parameters: no body"

        for pid in [ "C | ClicVolume"; "ClicVolume"; "M | MasterVolume" ] do
            let target =
                if pid.Contains "Master" then 1.0
                else value

            let! r = mcp.CallTool("set_parameter", $"""{{"id":"{pid}","value":{target}}}""")

            match r with
            | Some node -> printfn "set %s = %g → %s" pid target (node.ToJsonString().Substring(0, min 180 (node.ToJsonString().Length)))
            | None -> printfn "set %s = %g → (no body)" pid target

        let! got = mcp.CallTool("get_parameter", """{"id":"C | ClicVolume"}""")

        match got with
        | Some node -> printfn "get C | ClicVolume → %s" (node.ToJsonString())
        | None -> printfn "get C | ClicVolume → (no body)"
    }

main |> Async.AwaitTask |> Async.RunSynchronously
