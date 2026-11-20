//! Task fixtures and oracle harness builders for MENS v9.4 heldout eval v2.

use anyhow::{Context, Result};
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct V2TaskSpec {
    pub id: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub signature: &'static str,
    pub expected: &'static [&'static str],
    pub answer: &'static str,
    pub harness: &'static str,
    pub wrong_stub: &'static str,
}

#[derive(Clone)]
pub(super) struct V2OwnedTask {
    pub id: String,
    pub category: String,
    pub description: String,
    pub semantic_expected_contains: Vec<String>,
    pub answer: String,
    pub test_harness: String,
    pub wrong_stub: String,
}

impl From<V2TaskSpec> for V2OwnedTask {
    fn from(s: V2TaskSpec) -> Self {
        Self {
            id: s.id.to_string(),
            category: s.category.to_string(),
            description: format!("{}\n\nRequired signature:\n{}", s.description, s.signature),
            semantic_expected_contains: s.expected.iter().map(|x| (*x).to_string()).collect(),
            answer: s.answer.to_string(),
            test_harness: s.harness.to_string(),
            wrong_stub: s.wrong_stub.to_string(),
        }
    }
}

/// Loads all `training_eligible: false` problems from `contracts/eval/humaneval-vox`
/// and turns each problem's `tests.vox` `fn main(` block into `fn __vox_eval_harness(`.
pub(super) fn load_humaneval_heldout_v2(root: &Path) -> Result<Vec<V2OwnedTask>> {
    let corpus_root = root.join("contracts/eval/humaneval-vox");
    let fixtures = vox_corpus::humaneval_runner::manifest::load_corpus(&corpus_root)?;
    let mut out = Vec::new();
    for fx in fixtures {
        if fx.training_eligible {
            continue;
        }
        let reference = std::fs::read_to_string(&fx.reference_path)
            .with_context(|| format!("reading {}", fx.reference_path.display()))?;
        let tests_src = std::fs::read_to_string(&fx.tests_path)
            .with_context(|| format!("reading {}", fx.tests_path.display()))?;
        let harness_main = vox_corpus::humaneval_runner::compose::extract_test_block(&tests_src)?;
        let test_harness = rewrite_main_to_eval_harness(&harness_main);
        let fn_name = extract_fn_name_from_sig(&fx.signature)
            .with_context(|| format!("extracting fn name from `{}`", fx.signature))?;
        let wrong_stub = build_wrong_stub_from_signature(&fx.signature);
        let id = format!("humaneval_vox_{}", fx.slug.replace('-', "_"));
        let description = format!("{}\n\nRequired signature:\n{}", fx.prompt, fx.signature);
        out.push(V2OwnedTask {
            id,
            category: "humaneval-vox".to_string(),
            description,
            semantic_expected_contains: vec![format!("fn {fn_name}")],
            answer: reference,
            test_harness,
            wrong_stub,
        });
    }
    Ok(out)
}

pub(super) fn rewrite_main_to_eval_harness(main_block: &str) -> String {
    if let Some(pos) = main_block.find("fn main(") {
        let mut s = String::with_capacity(main_block.len() + 16);
        s.push_str(&main_block[..pos]);
        s.push_str("fn __vox_eval_harness(");
        s.push_str(&main_block[pos + "fn main(".len()..]);
        s
    } else {
        main_block.to_string()
    }
}

pub(super) fn extract_fn_name_from_sig(sig: &str) -> Option<String> {
    let rest = sig.trim().strip_prefix("fn ")?.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

fn build_wrong_stub_from_signature(sig: &str) -> String {
    let sig_t = sig.trim().trim_end_matches('{').trim();
    let ret_ty = sig_t.rsplit(") to ").next().map(str::trim).unwrap_or("int");
    let ret_expr = match ret_ty {
        "int" => "-999999",
        "float" => "-999.5",
        "bool" => "false",
        "str" => "\"__wrong__\"",
        "list[int]" => "[999999, -999999]",
        "list[list[int]]" => "[[999999]]",
        "list[str]" => "[\"__wrong__\"]",
        _ => "0",
    };
    format!("{sig_t} {{\n    return {ret_expr}\n}}")
}

/// All 15 `function` tasks from v1 `heldout_bench/manifest.json`, upgraded with
/// explicit signatures, working Vox reference code, and multi-case assertions.
pub(super) fn v1_function_tasks() -> Vec<V2TaskSpec> {
    vec![
        V2TaskSpec {
            id: "count_multiples_of_3_or_5",
            category: "function",
            description: "define a function named `count_multiples_of_3_or_5` that takes an integer `n` and returns the sum of all positive multiples of 3 or 5 strictly below `n`",
            signature: "fn count_multiples_of_3_or_5(n: int) to int",
            expected: &["fn count_multiples_of_3_or_5"],
            answer: "fn count_multiples_of_3_or_5(n: int) to int {\n    let mut sum: int = 0\n    let mut i: int = 1\n    while i < n {\n        if i % 3 == 0 or i % 5 == 0 {\n            sum = sum + i\n        }\n        i = i + 1\n    }\n    return sum\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(count_multiples_of_3_or_5(10) == 23)\n    assert(count_multiples_of_3_or_5(16) == 60)\n    assert(count_multiples_of_3_or_5(1) == 0)\n    assert(count_multiples_of_3_or_5(0) == 0)\n    return \"ok\"\n}",
            wrong_stub: "fn count_multiples_of_3_or_5(n: int) to int {\n    return -999999\n}",
        },
        V2TaskSpec {
            id: "matrix_row_sums",
            category: "function",
            description: "define a function named `matrix_row_sums` that takes a list of lists of integers `matrix` and returns a list of the sum of each row",
            signature: "fn matrix_row_sums(matrix: list[list[int]]) to list[int]",
            expected: &["fn matrix_row_sums"],
            answer: "fn matrix_row_sums(matrix: list[list[int]]) to list[int] {\n    let mut sums: list[int] = []\n    let mut i: int = 0\n    while i < matrix.len() {\n        let row: list[int] = matrix[i].unwrap()\n        let mut s: int = 0\n        let mut j: int = 0\n        while j < row.len() {\n            s = s + row[j].unwrap()\n            j = j + 1\n        }\n        sums = sums.push(s)\n        i = i + 1\n    }\n    return sums\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(matrix_row_sums([[1, 2, 3], [4, 5, 6]]) == [6, 15])\n    assert(matrix_row_sums([[-2, 5], [0, 0], [10]]) == [3, 0, 10])\n    assert(matrix_row_sums([]) == [])\n    return \"ok\"\n}",
            wrong_stub: "fn matrix_row_sums(matrix: list[list[int]]) to list[int] {\n    return [999999]\n}",
        },
        V2TaskSpec {
            id: "matrix_col_sums",
            category: "function",
            description: "define a function named `matrix_col_sums` that takes a list of lists of integers `matrix` (all rows same length) and returns a list of the sum of each column",
            signature: "fn matrix_col_sums(matrix: list[list[int]]) to list[int]",
            expected: &["fn matrix_col_sums"],
            answer: "fn matrix_col_sums(matrix: list[list[int]]) to list[int] {\n    if matrix.len() == 0 {\n        return []\n    }\n    let mut sums: list[int] = []\n    let cols: int = matrix[0].unwrap().len()\n    let mut c: int = 0\n    while c < cols {\n        let mut s: int = 0\n        let mut r: int = 0\n        while r < matrix.len() {\n            s = s + matrix[r].unwrap()[c].unwrap()\n            r = r + 1\n        }\n        sums = sums.push(s)\n        c = c + 1\n    }\n    return sums\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(matrix_col_sums([[1, 2, 3], [4, 5, 6]]) == [5, 7, 9])\n    assert(matrix_col_sums([[-1, 10], [3, -4], [2, 1]]) == [4, 7])\n    assert(matrix_col_sums([[7, 8]]) == [7, 8])\n    return \"ok\"\n}",
            wrong_stub: "fn matrix_col_sums(matrix: list[list[int]]) to list[int] {\n    return [999999]\n}",
        },
        V2TaskSpec {
            id: "is_symmetric_matrix",
            category: "function",
            description: "define a function named `is_symmetric_matrix` that takes a square list of lists of integers `matrix` and returns true if matrix[i][j] equals matrix[j][i] for all i, j",
            signature: "fn is_symmetric_matrix(matrix: list[list[int]]) to bool",
            expected: &["fn is_symmetric_matrix"],
            answer: "fn is_symmetric_matrix(matrix: list[list[int]]) to bool {\n    let n: int = matrix.len()\n    let mut i: int = 0\n    while i < n {\n        let mut j: int = 0\n        while j < n {\n            if matrix[i].unwrap()[j].unwrap() != matrix[j].unwrap()[i].unwrap() {\n                return false\n            }\n            j = j + 1\n        }\n        i = i + 1\n    }\n    return true\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(is_symmetric_matrix([[1, 2, 3], [2, 5, 6], [3, 6, 9]]) == true)\n    assert(is_symmetric_matrix([[1, 0], [1, 1]]) == false)\n    assert(is_symmetric_matrix([[42]]) == true)\n    assert(is_symmetric_matrix([]) == true)\n    return \"ok\"\n}",
            wrong_stub: "fn is_symmetric_matrix(matrix: list[list[int]]) to bool {\n    return false\n}",
        },
        V2TaskSpec {
            id: "char_histogram",
            category: "function",
            description: "define type `CharCount` and a function named `char_histogram` that takes a string `s` and returns a list of `CharCount(ch, count)` values in first-appearance order for every distinct character in `s`",
            signature: "type CharCount =\n    | CharCount(ch: str, count: int)\n\nfn char_histogram(s: str) to list[CharCount]",
            expected: &["fn char_histogram"],
            answer: "type CharCount =\n    | CharCount(ch: str, count: int)\n\nfn char_histogram(s: str) to list[CharCount] {\n    let mut keys: list[str] = []\n    let mut counts: list[int] = []\n    let mut i: int = 0\n    while i < s.len() {\n        let ch: str = s.slice(i, i + 1)\n        let mut found: bool = false\n        let mut k: int = 0\n        while k < keys.len() {\n            if keys[k].unwrap() == ch {\n                counts[k] = counts[k].unwrap() + 1\n                found = true\n            }\n            k = k + 1\n        }\n        if not found {\n            keys = keys.push(ch)\n            counts = counts.push(1)\n        }\n        i = i + 1\n    }\n    let mut out: list[CharCount] = []\n    let mut m: int = 0\n    while m < keys.len() {\n        out = out.push(CharCount(keys[m].unwrap(), counts[m].unwrap()))\n        m = m + 1\n    }\n    return out\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(char_histogram(\"abca\") == [CharCount(\"a\", 2), CharCount(\"b\", 1), CharCount(\"c\", 1)])\n    assert(char_histogram(\"\") == [])\n    return \"ok\"\n}",
            wrong_stub: "type CharCount =\n    | CharCount(ch: str, count: int)\n\nfn char_histogram(s: str) to list[CharCount] {\n    return [CharCount(\"__wrong__\", -1)]\n}",
        },
        V2TaskSpec {
            id: "longest_increasing_run",
            category: "function",
            description: "define a function named `longest_increasing_run` that takes a list of integers `nums` and returns the length of the longest run of strictly increasing consecutive elements",
            signature: "fn longest_increasing_run(nums: list[int]) to int",
            expected: &["fn longest_increasing_run"],
            answer: "fn longest_increasing_run(nums: list[int]) to int {\n    if nums.len() == 0 {\n        return 0\n    }\n    let mut best: int = 1\n    let mut cur: int = 1\n    let mut i: int = 1\n    while i < nums.len() {\n        if nums[i].unwrap() > nums[i - 1].unwrap() {\n            cur = cur + 1\n        } else {\n            cur = 1\n        }\n        if cur > best {\n            best = cur\n        }\n        i = i + 1\n    }\n    return best\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(longest_increasing_run([1, 2, 3, 2, 3, 4, 5]) == 4)\n    assert(longest_increasing_run([5, 4, 3, 2]) == 1)\n    assert(longest_increasing_run([2, 2, 2]) == 1)\n    assert(longest_increasing_run([]) == 0)\n    return \"ok\"\n}",
            wrong_stub: "fn longest_increasing_run(nums: list[int]) to int {\n    return -999999\n}",
        },
        V2TaskSpec {
            id: "is_valid_ipv4",
            category: "function",
            description: "define a function named `is_valid_ipv4` that takes a string `s` and returns true if it is a valid dotted-quad IPv4 address (four dot-separated numeric octets each between 0 and 255)",
            signature: "fn is_valid_ipv4(s: str) to bool",
            expected: &["fn is_valid_ipv4"],
            answer: "fn is_valid_ipv4(s: str) to bool {\n    let parts: list[str] = s.split(\".\")\n    if parts.len() != 4 {\n        return false\n    }\n    let mut i: int = 0\n    while i < 4 {\n        let part: str = parts[i].unwrap()\n        if not part.is_digit() {\n            return false\n        }\n        match part.to_int() {\n            Some(n) => {\n                if n < 0 or n > 255 {\n                    return false\n                }\n            }\n            None => { return false }\n        }\n        i = i + 1\n    }\n    return true\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(is_valid_ipv4(\"192.168.1.1\") == true)\n    assert(is_valid_ipv4(\"0.0.0.0\") == true)\n    assert(is_valid_ipv4(\"256.1.1.1\") == false)\n    assert(is_valid_ipv4(\"1.2.3\") == false)\n    assert(is_valid_ipv4(\"1.2.abc.4\") == false)\n    return \"ok\"\n}",
            wrong_stub: "fn is_valid_ipv4(s: str) to bool {\n    return false\n}",
        },
        V2TaskSpec {
            id: "string_is_rotation",
            category: "function",
            description: "define a function named `string_is_rotation` that takes two strings `a` and `b` of equal length and returns true if `b` is a rotation of `a`",
            signature: "fn string_is_rotation(a: str, b: str) to bool",
            expected: &["fn string_is_rotation"],
            answer: "fn string_is_rotation(a: str, b: str) to bool {\n    if a.len() != b.len() {\n        return false\n    }\n    let doubled: str = a + a\n    return doubled.contains(b)\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(string_is_rotation(\"waterbottle\", \"erbottlewat\") == true)\n    assert(string_is_rotation(\"abc\", \"acb\") == false)\n    assert(string_is_rotation(\"ab\", \"abc\") == false)\n    assert(string_is_rotation(\"\", \"\") == true)\n    return \"ok\"\n}",
            wrong_stub: "fn string_is_rotation(a: str, b: str) to bool {\n    return false\n}",
        },
        V2TaskSpec {
            id: "balanced_multi_brackets",
            category: "function",
            description: "define a function named `balanced_multi_brackets` that takes a string `s` containing `()`, `[]`, and `{}` and returns true if every bracket type is properly matched and nested",
            signature: "fn balanced_multi_brackets(s: str) to bool",
            expected: &["fn balanced_multi_brackets"],
            answer: "fn balanced_multi_brackets(s: str) to bool {\n    let mut stack: list[str] = []\n    let chars: list[str] = s.chars()\n    let mut i: int = 0\n    while i < chars.len() {\n        let ch: str = chars[i].unwrap()\n        if ch == \"(\" or ch == \"[\" or ch == \"{\" {\n            stack = stack.push(ch)\n        } else if ch == \")\" or ch == \"]\" or ch == \"}\" {\n            if stack.len() == 0 {\n                return false\n            }\n            let top: str = stack[stack.len() - 1].unwrap()\n            let matches: bool = (ch == \")\" and top == \"(\") or (ch == \"]\" and top == \"[\") or (ch == \"}\" and top == \"{\")\n            if not matches {\n                return false\n            }\n            stack = stack.remove_at(stack.len() - 1)\n        }\n        i = i + 1\n    }\n    return stack.len() == 0\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(balanced_multi_brackets(\"{[()]}\") == true)\n    assert(balanced_multi_brackets(\"{[(])}\") == false)\n    assert(balanced_multi_brackets(\"(((\") == false)\n    assert(balanced_multi_brackets(\"()[]{}\") == true)\n    assert(balanced_multi_brackets(\"\") == true)\n    return \"ok\"\n}",
            wrong_stub: "fn balanced_multi_brackets(s: str) to bool {\n    return false\n}",
        },
        V2TaskSpec {
            id: "count_words_starting_with",
            category: "function",
            description: "define a function named `count_words_starting_with` that takes a string `s` and a single-character string `letter`, and returns the count of whitespace-separated words in `s` that start with `letter`",
            signature: "fn count_words_starting_with(s: str, letter: str) to int",
            expected: &["fn count_words_starting_with"],
            answer: "fn count_words_starting_with(s: str, letter: str) to int {\n    let words: list[str] = s.split(\" \")\n    let mut count: int = 0\n    let mut i: int = 0\n    while i < words.len() {\n        let w: str = words[i].unwrap()\n        if w.len() > 0 and w.slice(0, 1) == letter {\n            count = count + 1\n        }\n        i = i + 1\n    }\n    return count\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(count_words_starting_with(\"alpha beta apricot\", \"a\") == 2)\n    assert(count_words_starting_with(\"  one  two  \", \"t\") == 1)\n    assert(count_words_starting_with(\"\", \"a\") == 0)\n    return \"ok\"\n}",
            wrong_stub: "fn count_words_starting_with(s: str, letter: str) to int {\n    return -999999\n}",
        },
        V2TaskSpec {
            id: "weighted_average",
            category: "function",
            description: "define a function named `weighted_average` that takes a list of integers `values` and a list of integers `weights` of the same length and returns the weighted average as a float",
            signature: "fn weighted_average(values: list[int], weights: list[int]) to float",
            expected: &["fn weighted_average"],
            answer: "fn weighted_average(values: list[int], weights: list[int]) to float {\n    let mut total: float = 0.0\n    let mut weight_sum: float = 0.0\n    let mut i: int = 0\n    while i < values.len() {\n        let v: float = float(values[i].unwrap())\n        let w: float = float(weights[i].unwrap())\n        total = total + v * w\n        weight_sum = weight_sum + w\n        i = i + 1\n    }\n    return total / weight_sum\n}",
            harness: "fn __vox_eval_harness() to str {\n    let a: float = weighted_average([10, 20], [1, 3])\n    assert(a > 17.49 and a < 17.51)\n    let b: float = weighted_average([5, 5, 5], [2, 2, 2])\n    assert(b > 4.99 and b < 5.01)\n    let c: float = weighted_average([0, 100], [9, 1])\n    assert(c > 9.99 and c < 10.01)\n    return \"ok\"\n}",
            wrong_stub: "fn weighted_average(values: list[int], weights: list[int]) to float {\n    return -999.5\n}",
        },
        V2TaskSpec {
            id: "dedupe_case_insensitive",
            category: "function",
            description: "define a function named `dedupe_case_insensitive` that takes a list of strings `items` and returns a new list with case-insensitive duplicates removed, keeping the first occurrence",
            signature: "fn dedupe_case_insensitive(items: list[str]) to list[str]",
            expected: &["fn dedupe_case_insensitive"],
            answer: "fn dedupe_case_insensitive(items: list[str]) to list[str] {\n    let mut seen: list[str] = []\n    let mut out: list[str] = []\n    let mut i: int = 0\n    while i < items.len() {\n        let lower: str = items[i].unwrap().to_lower()\n        if not seen.contains(lower) {\n            seen = seen.push(lower)\n            out = out.push(items[i].unwrap())\n        }\n        i = i + 1\n    }\n    return out\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(dedupe_case_insensitive([\"Alpha\", \"alpha\", \"Beta\", \"BETA\", \"Gamma\"]) == [\"Alpha\", \"Beta\", \"Gamma\"])\n    assert(dedupe_case_insensitive([]) == [])\n    assert(dedupe_case_insensitive([\"x\", \"y\"]) == [\"x\", \"y\"])\n    return \"ok\"\n}",
            wrong_stub: "fn dedupe_case_insensitive(items: list[str]) to list[str] {\n    return [\"__wrong__\"]\n}",
        },
        V2TaskSpec {
            id: "sum_of_even_indexed",
            category: "function",
            description: "define a function named `sum_of_even_indexed` that takes a list of integers `nums` and returns the sum of the elements at even indices (0, 2, 4, ...)",
            signature: "fn sum_of_even_indexed(nums: list[int]) to int",
            expected: &["fn sum_of_even_indexed"],
            answer: "fn sum_of_even_indexed(nums: list[int]) to int {\n    let mut sum: int = 0\n    let mut i: int = 0\n    while i < nums.len() {\n        sum = sum + nums[i].unwrap()\n        i = i + 2\n    }\n    return sum\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(sum_of_even_indexed([10, 99, 20, 88, 30]) == 60)\n    assert(sum_of_even_indexed([7]) == 7)\n    assert(sum_of_even_indexed([]) == 0)\n    return \"ok\"\n}",
            wrong_stub: "fn sum_of_even_indexed(nums: list[int]) to int {\n    return -999999\n}",
        },
        V2TaskSpec {
            id: "max_consecutive_ones",
            category: "function",
            description: "define a function named `max_consecutive_ones` that takes a list of integers `bits` (each 0 or 1) and returns the length of the longest run of consecutive 1s",
            signature: "fn max_consecutive_ones(bits: list[int]) to int",
            expected: &["fn max_consecutive_ones"],
            answer: "fn max_consecutive_ones(bits: list[int]) to int {\n    let mut best: int = 0\n    let mut cur: int = 0\n    let mut i: int = 0\n    while i < bits.len() {\n        if bits[i].unwrap() == 1 {\n            cur = cur + 1\n        } else {\n            cur = 0\n        }\n        if cur > best {\n            best = cur\n        }\n        i = i + 1\n    }\n    return best\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(max_consecutive_ones([1, 1, 0, 1, 1, 1]) == 3)\n    assert(max_consecutive_ones([0, 0, 0]) == 0)\n    assert(max_consecutive_ones([]) == 0)\n    return \"ok\"\n}",
            wrong_stub: "fn max_consecutive_ones(bits: list[int]) to int {\n    return -999999\n}",
        },
        V2TaskSpec {
            id: "interleave_lists",
            category: "function",
            description: "define a function named `interleave_lists` that takes two lists of integers `a` and `b` of equal length and returns a single list alternating elements from `a` and `b`, starting with `a`",
            signature: "fn interleave_lists(a: list[int], b: list[int]) to list[int]",
            expected: &["fn interleave_lists"],
            answer: "fn interleave_lists(a: list[int], b: list[int]) to list[int] {\n    let mut out: list[int] = []\n    let mut i: int = 0\n    while i < a.len() {\n        out = out.push(a[i].unwrap())\n        out = out.push(b[i].unwrap())\n        i = i + 1\n    }\n    return out\n}",
            harness: "fn __vox_eval_harness() to str {\n    assert(interleave_lists([1, 3, 5], [2, 4, 6]) == [1, 2, 3, 4, 5, 6])\n    assert(interleave_lists([], []) == [])\n    assert(interleave_lists([9], [8]) == [9, 8])\n    return \"ok\"\n}",
            wrong_stub: "fn interleave_lists(a: list[int], b: list[int]) to list[int] {\n    return [999999]\n}",
        },
    ]
}
