pub fn vox_grammar_prompt() -> &'static str {
    r#"
Vox Language Syntax (v0.3 — AI-native edition)

== Functions ==
fn name(arg: Type) -> ReturnType { ... }
async fn fetch() -> Result[str] { ... }
pub fn exported() -> Unit { ... }

== Variables ==
let x = 10
let mut y = 20

== Control Flow ==
if cond { ... } else { ... }
while cond { ... }
loop { break; continue; }
return value

== Types ==
int, float, str, bool, Unit
List[T], Option[T], Result[T]
(T1, T2)

== Decorators ==
@test fn my_test() { ... }
@server fn api_call() -> Result[str] { ... }
@route("GET", "/path") fn handler() -> Result[Res] { ... }
@mutation fn save(item: Item) -> Result[Unit] { ... }
@query fn fetch_items() -> Result[List[Item]] { ... }

== Components (Path C — canonical) ==
component Counter() {
    state count: int = 0
    view: div {
        p { "Count: {count}" }
        button onclick=|| { count += 1 } { "+" }
    }
}

@component fn MyComponent() -> Element { ... }   // DEPRECATED — use component Name() { } above

== Tables ==
@table
type Post {
    id: int
    title: str
    body: str
}

== Agents ==
agent MyAgent {
    on message(msg: str) {
        let result = ai.complete(msg)
        return result
    }
}
"#
}
