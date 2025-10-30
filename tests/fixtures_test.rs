use libtest_mimic::{Arguments, Trial};
use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    io,
    path::{Path, PathBuf},
};

use assert_cmd::Command;

// TODO: These tests fail in debug but works in release
// due to some newline shenanigans, we should investigate why
#[cfg(debug_assertions)]
const IGNORED_IN_DEBUG: &[&'static str] = &[
    "test_ripper_large_concurrent_ruby_future",
    "test_ripper_large_rspec_mocks_proxy",
    "test_ripper_large_concurrent-ruby_non_concurrent_map_backend",
    "test_prism_large_concurrent_ruby_future",
    "test_prism_large_rspec_mocks_proxy",
    "test_prism_large_concurrent-ruby_non_concurrent_map_backend",
];

#[cfg(not(debug_assertions))]
const IGNORED_IN_DEBUG: &[&'static str] = &[];

// These tests are disabled due to ripper issues
const DISABLED_RIPPER_TESTS: &[&'static str] = &[
    // TODO: The Ripper implementation does not currently support args forwarding with additional
    // arguments passed in.
    // https://github.com/fables-tales/rubyfmt/issues/474
    "small_args_forwarding_additional_args",
];

// These tests will have their prism variants automatically ignored
const DISABLED_PRISM_TESTS: &[&'static str] = &[
    "large_concurrent-ruby_synchronized_map_backend",
    "large_concurrent-ruby_atom",
    "large_concurrent-ruby_ruby_non_concurrent_priority_queue",
    "large_concurrent-ruby_truffleruby_map_backend",
    "large_concurrent-ruby_numeric_cas_wrapper",
    "large_concurrent-ruby_non_concurrent_priority_queue",
    "large_concurrent-ruby_copy_on_notify_observer_set",
    "large_concurrent-ruby_ivar",
    "large_concurrent-ruby_mutex_atomic",
    "large_dqt",
    "large_rspec_core_notifications",
    "large_concurrent-ruby_java_non_concurrent_priority_queue",
    "small_2.5_lambda_do_end",
    "small_list_like_things_with_comments",
    "small_aref_in_call",
    "small_backref",
    "small_cvar",
    "small_unless_mod",
    "small_conditional",
    "small_multiline_chain_in_block",
    "small_alias_bare_kw",
    "small_not",
    "small_unless",
    "small_multiline_when_with_comment",
    "small_cannibalization_1",
    "small_comments_at_indentation_changes",
    "small_percent_q",
    "small_gemfile",
    "small_alias",
    "small_bitwise_not",
    "small_class_module",
    "small_visibility_modifier",
    "small_and_or",
    "small_character_literal",
    "small_mrhs_new_from_args",
    "small_aref_rest_param",
    "small_string_literal_with_class_interp",
    "small_const_call",
    "small_string_dvar",
    "small_massign_omg",
    "small_method_chains",
    "small_until_while_mod",
    "small_string_with_embexpr_dyna_symbol",
    "small_w_array",
    "small_many_opassigns",
    "small_break_all_the_things",
    "small_rspec_its",
    "small_binary_raise",
    "small_more_methods",
    "small_class_methods_respect_parens",
    "small_w_arrays",
    "small_method_annotation",
    "small_many_weird_args",
    "small_stabby_lambda",
    "small_nested_destructuring",
    "small_bare_rescue_comments",
    "small_procs",
    "small_unary",
    "small_comments_with_breaks",
    "small_multi_rescue",
    "small_scoping",
    "small_mlhs_params",
    "small_case_else",
    "small_while",
    "small_pathological_heredocs",
    "small_fib",
    "small_alias_equal",
    "small_variable_binding",
    "small_sclass",
    "small_inline_rescue",
    "small_complex_numbers",
    "small_mrhs_add_star",
    "small_single_massign",
    "small_string_first_item_is_embed",
    "small_rationals",
    "small_long_raise",
    "small_require_rails",
    "small_for_loop",
    "small_empty_heredocs",
    "small_alias_string_symbol",
    "small_require_relative",
    "small_bracket_method_call_on_module_with_no_args",
    "small_block_param_line_length",
    "small_when_indent",
    "small_heredocs_ending_blocks",
    "small_hash_rocket_usage",
    "small_defined",
    "small_while_block_comment",
    "small_splat_case",
    "small_mlhs_paren",
    "small_begin_block",
    "small_multi_line_word_arrays",
    "small_string_literals_dont_break_comments",
    "small_heredoc_ending_with_curly",
    "small_undef",
    "small_return0",
    "small_conditional_assign",
    "small_massign",
    "small_command_call_raise",
    "small_aref_field",
    "small_bare_alias",
    "small_mrhs_add_star_2",
    "small_conditionals_with_ending_comments",
    "small_rest_param_unpacking",
    "small_binary_operators",
    "small_breakable_binary_op",
    "small_case_multi",
    "small_while_mod",
    "small_yield_with_paren",
    "small_lambda",
    "small_multi_assign_method",
    "small_empty_arg_paren",
    "small_cursed_call_01",
    "small_assign_field",
    "small_end_block",
    "small_dyna_symbol_with_escapes",
    "small_paren_expr_calls",
    "small_breaks",
    "small_binary_line_splitting",
    "small_return",
    "small_breakables_over_line_length",
    "small_opassign",
    "small_conditional_expanded_chain",
    "small_brace_blocks_with_no_args",
    "small_until",
    "small_dotcall",
    "small_multiline_method_chain_with_arguments",
    "small_heredocs",
    "small_yield_hash_args",
    "small_case",
    "small_block_argument",
];

fn main() -> io::Result<()> {
    let args = Arguments::from_args();

    let mut tests = Vec::new();

    let fixtures = collect_fixtures("fixtures".into())?;

    tests.extend(fixtures.iter().map(|f| f.to_trial(Flavor::Ripper)));
    tests.extend(fixtures.iter().map(|f| f.to_trial(Flavor::Prism)));

    libtest_mimic::run(&args, tests).exit();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Flavor {
    Ripper,
    Prism,
}

impl Flavor {
    fn to_str(&self) -> &'static str {
        match self {
            Flavor::Ripper => "ripper",
            Flavor::Prism => "prism",
        }
    }

    fn disabled_list(&self) -> &[&'static str] {
        match self {
            Flavor::Ripper => DISABLED_RIPPER_TESTS,
            Flavor::Prism => DISABLED_PRISM_TESTS,
        }
    }
}

#[derive(Debug)]
struct Fixture {
    name: String,
    actual: Option<PathBuf>,
    expected: Option<PathBuf>,
}

impl Fixture {
    fn to_trial(&self, flavor: Flavor) -> Trial {
        let name = format!("test_{}_{}", flavor.to_str(), self.name);
        let is_ignored = IGNORED_IN_DEBUG.contains(&name.as_str())
            || flavor.disabled_list().contains(&self.name.as_str());
        let actual = self.actual.clone();
        let expected = self.expected.clone();
        Trial::test(name.clone(), move || {
            let Some(actual) = actual else {
                return Result::Err(format!("Test {} is missing an _actual.rb file", name).into());
            };

            let Some(expected) = expected else {
                return Result::Err(
                    format!("Test {} is missing an _expected.rb file", name).into(),
                );
            };

            let expected_text = read_to_string(&expected)?;

            // Test if the formatting works as expected
            let mut cmd = Command::cargo_bin("rubyfmt-main").unwrap();
            cmd.arg(actual.to_str().unwrap());

            if flavor == Flavor::Prism {
                cmd.arg("--prism");
            }

            cmd.assert().success().stdout(expected_text.clone());

            // Test if the formatting is idempotent
            let mut cmd = Command::cargo_bin("rubyfmt-main").unwrap();
            cmd.arg(expected.to_str().unwrap());

            if flavor == Flavor::Prism {
                cmd.arg("--prism");
            }

            cmd.assert().success().stdout(expected_text);
            Ok(())
        })
        .with_ignored_flag(is_ignored)
    }
}

fn collect_fixtures(path: PathBuf) -> io::Result<Vec<Fixture>> {
    #[derive(Default)]
    struct Partial {
        actual: Option<PathBuf>,
        expected: Option<PathBuf>,
    }

    fn recurse(results: &mut Vec<Fixture>, path: &Path) -> io::Result<()> {
        let mut partial: HashMap<String, Partial> = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                recurse(results, &path)?;
                continue;
            }

            let file = path.to_str().unwrap();
            if file.ends_with("_actual.rb") {
                let len = file.len() - "_actual.rb".len();
                let prefix = file[0..len].to_owned();
                let p = partial.entry(prefix).or_default();
                assert!(p.actual.is_none());
                p.actual = Some(path);
            } else if file.ends_with("_expected.rb") {
                let len = file.len() - "_expected.rb".len();
                let prefix = file[0..len].to_owned();
                let p = partial.entry(prefix).or_default();
                assert!(p.expected.is_none());
                p.expected = Some(path);
            }
        }

        for (mut k, v) in partial {
            k.drain(0.."fixtures/".len());
            results.push(Fixture {
                name: k.replace('/', "_"),
                actual: v.actual,
                expected: v.expected,
            });
        }

        Ok(())
    }

    let mut results = Vec::new();
    recurse(&mut results, &path)?;
    Ok(results)
}
