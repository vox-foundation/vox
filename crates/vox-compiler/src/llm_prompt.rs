pub fn vox_grammar_prompt() -> &'static str {
    r#"
Vox Language Syntax Cheat Sheet

Declarations:
fn name(arg: type) -> type { ... }
async fn request() -> Result[str] { ... }
let x = 10
let mut y = 20

Control Flow:
if cond { ... } else { ... }
while cond { ... }
loop { break; continue; }
return value
ret value

Types:
int, float, str, bool, Unit
List[T], Option[T], Result[T]
(T1, T2)

Decorators:
@test fn my_test() { ... }
@component fn MyComponent() -> Element { ... }
@server fn api_call() -> Result[str] { ... }
"#
}
