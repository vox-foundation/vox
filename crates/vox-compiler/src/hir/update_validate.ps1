$file = "c:\Users\Owner\vox\crates\vox-compiler\src\hir\validate.rs"
$content = Get-Content $file -Raw

# Update struct definition
$content = $content -replace 'pub span: Span,', "pub span: Span,`n    pub correction_hint: Option<String>,"

# Update all instantiations
$content = $content -replace "span: ([a-zA-Z0-9_\.]+),`r`n\s+`}", "span: `$1,`n                correction_hint: None,`n            }"
$content = $content -replace "span: ([a-zA-Z0-9_\.]+),`n\s+`}", "span: `$1,`n                correction_hint: None,`n            }"

Set-Content -Path $file -Value $content -NoNewline
