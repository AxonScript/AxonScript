// AxonScript CLI


mod ast;
mod compiler_neuron;
mod high_level_ir;
mod lexer_tokenizer;
mod parser;
mod semantic;

use crate::compiler_neuron::{compile_and_run_jit, create_llvm_module, emit_object_file, CompilerError};
use crate::high_level_ir::HIRStatement;
use crate::lexer_tokenizer::lex_with_span;
use crate::parser::parser_error::{ErrorKind, ParseError, Severity};
use crate::parser::parser_kernel::Parser as AxonParser;
use crate::semantic::{ast_to_hir, semantic_error::SemanticError};
use console::style;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{stdout, Write};
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use target_lexicon::HOST;

const VERSION: &str = "AxonScript Build #0001 Pre-Alpha Demo";
const WEBSITE: &str = "https://axonscript.org";

impl From<&SemanticError> for ParseError {
    fn from(e: &SemanticError) -> Self {
        ParseError {
            kind: ErrorKind::Semantic,
            message: e.message.clone(),
            start: e.start,
            end: e.end,
            src: e.src.clone(),
            suggestion: None,
            severity: Severity::Error,
        }
    }
}

impl ParseError {
    pub fn from_compiler_error(e: &CompilerError, src: &str) -> Self {
        ParseError::new(
            ErrorKind::Codegen,
            e.0.clone(),
            0,
            0,
            Some(src.to_string()),
            None,
            Severity::Error,
        )
    }
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        print_help();
        return;
    }
    match args[1].as_str() {
        "create" if args.len() >= 4 && args[2] == "project" => {
            let name = args[3..].join(" ");
            if name.is_empty() {
                println!("Project name cannot be empty!");
                return;
            }
            if Path::new(&name).exists() {
                print_error(
                    "Create",
                    &[ParseError::new(
                        ErrorKind::Syntax,
                        format!("Project '{}' already exists.", name),
                        0,
                        0,
                        None,
                        None,
                        Severity::Error,
                    )],
                );
                return;
            }
            let src_path = format!("{}/src", name);
            let project_path = format!("{}/project.asml", name);
            let ax_path = format!("{}/init.ax", src_path);
            fs::create_dir_all(&src_path).unwrap();
            let mut project_file = File::create(&project_path).unwrap();
            writeln!(project_file, "__Project__").unwrap();
            writeln!(project_file, "_name_ = \"{}\"", name).unwrap();
            writeln!(project_file, "_version_ = \"0.1.0\"").unwrap();
            let mut main_ax = File::create(&ax_path).unwrap();
            writeln!(main_ax, "cast Start() >>\nout(\"Hello World!\");\n<<").unwrap();
            println!(
                "{} Project '{}' created successfully!",
                style("✔").green().bold(),
                style(&name).yellow().bold()
            );
        }
        "create" if args.len() >= 4 && (args[2] == "ai" || args[2] == "pack") => {
            println!("{} Coming soon!", style("🚀").yellow().bold());
        }
        "build" => {
            let mut output_filename = None;
            if let Some(pos) = args.iter().position(|r| r == "--output") {
                if let Some(filename) = args.get(pos + 1) {
                    output_filename = Some(filename.clone());
                } else {
                    println!("{}", style("Error: The --output flag requires a filename.").red());
                    return;
                }
            }

            let mut target = None;
            if let Some(pos) = args.iter().position(|r| r == "--target") {
                if let Some(target_val) = args.get(pos + 1) {
                    target = Some(target_val.to_lowercase());
                } else {
                    println!(
                        "{}",
                        style("Error: The --target flag requires a value (e.g., windows, linux).")
                            .red()
                    );
                    return;
                }
            }
            run_pipeline("build", output_filename, target);
        }
        "run" | "check" => run_pipeline(&args[1], None, None),
        "--help" | "-h" => print_help(),
        "--version" | "-v" => println!("{}\nDocs: {}\n", VERSION, WEBSITE),
        _ => print_error(
            "Invalid",
            &[ParseError::new(
                ErrorKind::Syntax,
                "Unknown command. Use: axon --help".to_string(),
                0,
                0,
                None,
                None,
                Severity::Error,
            )],
        ),
    }
}

fn print_help() {
    println!(
        "{}\n\nUsage:\n  axon create project <name>      Create new AxonScript project\n  axon create ai <name>           Create new AI [coming soon]\n  axon create pack <name>         Create new package [coming soon]\n  axon install                    Install package [coming soon]\n  axon run                        Run project\n  axon build [--output <f>] [--target <os>] Build project\n  axon check                      Check syntax\n  axon test                       Run tests\n\nOptions:\n  --output <file>                 Specify output file name for build\n  --target <os>                   Specify target OS for build (windows, linux)\n  --help, -h                      Show help\n  --version, -v                   Show version\n\nDocs: {}\nCommunity: {}",
        style("AxonScript CLI").cyan().bold(),
        format!("{}/docs", WEBSITE),
        format!("{}/community", WEBSITE)
    );
}

fn link_object_file(obj_path: &str, exe_path: &str, target_triple: &str) -> Result<(), String> {
    let linker_cmd = if target_triple.contains("windows-msvc") {
        "link.exe"
    } else if target_triple.contains("windows-gnu") {
        "x86_64-w64-mingw32-gcc"
    } else {
        "cc"
    };

    let mut command = Command::new(linker_cmd);
    command.args([obj_path, "-o", exe_path]);

    if target_triple.contains("windows-gnu") {
        command.arg("-static");
    }

    let result = command.status();

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("Linker exited with status: {}", status)),
        Err(e) => Err(format!("Failed to execute linker: {}. Is a system linker (like GCC or MSVC Build Tools) in your PATH?", e)),
    }
}



fn run_pipeline(cmd: &str, output_filename: Option<String>, target: Option<String>) {
    clear_screen();
    print_header();

    let project_dir = Path::new(".");
    let asml_path = project_dir.join("project.asml");
    let src_file = project_dir.join("src/init.ax");
    if !asml_path.exists() || !src_file.exists() {
        print_error(
            "Setup",
            &[ParseError::new(
                ErrorKind::Syntax,
                format!("No project found in '{}'.", project_dir.display()),
                0,
                0,
                None,
                None,
                Severity::Error,
            )],
        );
        return;
    }

    let pipeline_run: &[&str] = &["Lexer", "Parser", "Semantic", "IR Codegen"];
    let pipeline_build: &[&str] = &["Lexer", "Parser", "Semantic", "IR Codegen", "Obj Gen", "Linking"];
    let pipeline = if cmd == "build" { pipeline_build } else { pipeline_run };

    print_progress(0, pipeline);
    let mut stage = 0;

    let code = match fs::read_to_string(&src_file) {
        Ok(c) => c,
        Err(_) => {
            print_error(
                "Lexing",
                &[ParseError::new(
                    ErrorKind::Syntax,
                    "Source not found".to_string(),
                    0,
                    0,
                    None,
                    None,
                    Severity::Error,
                )],
            );
            return;
        }
    };

    sleep(Duration::from_millis(100));
    let tokens = lex_with_span(&code);
    stage += 1;
    print_progress(stage, pipeline);

    sleep(Duration::from_millis(100));
    let mut parser = AxonParser {
        tokens: &tokens,
        pos: 0,
        src: Some(code.clone()),
    };
    let parse_result = parser.parse_program();
    let (parse_warnings, parse_errors): (Vec<_>, Vec<_>) = parse_result
        .errors
        .into_iter()
        .partition(|e| matches!(e.severity, Severity::Warning));

    if !parse_warnings.is_empty() {
        print_warning("Parsing", &parse_warnings);
    }
    if !parse_errors.is_empty() {
        print_error("Parsing", &parse_errors);
        return;
    }
    let ast = match parse_result.result {
        Some(ast) => ast,
        None => {
            print_error(
                "Parsing",
                &[ParseError::new(
                    ErrorKind::Syntax,
                    "Invalid AST".to_string(),
                    0,
                    0,
                    Some(code.clone()),
                    Some("Check syntax".to_string()),
                    Severity::Error,
                )],
            );
            return;
        }
    };
    stage += 1;
    print_progress(stage, pipeline);

    sleep(Duration::from_millis(100));
    let sem_result = ast_to_hir(ast, Some(code.clone()));
    let sem_parse_errs: Vec<ParseError> =
        sem_result.errors.iter().map(ParseError::from).collect();
    let (sem_warnings, sem_errors): (Vec<_>, Vec<_>) =
        sem_parse_errs.into_iter().partition(|e| matches!(e.severity, Severity::Warning));

    if !sem_warnings.is_empty() {
        print_warning("Semantic", &sem_warnings);
    }
    if !sem_errors.is_empty() {
        print_error("Semantic", &sem_errors);
        return;
    }
    let hir: Vec<HIRStatement> = sem_result.result;
    let mut_vars: HashSet<String> = sem_result.mutable_vars.clone();
    stage += 1;
    print_progress(stage, pipeline);

    if cmd == "check" {
        println!();
        println!("{} Syntax check passed successfully!", style("✔").green().bold());
        println!("💻 Community: {}/community", WEBSITE);
        return;
    }
    
    sleep(Duration::from_millis(100));

    if cmd == "build" {
        let compiler = match create_llvm_module(hir, mut_vars) {
            Ok(c) => c,
            Err(errors) => {
                print_error("IR Codegen", &errors.iter().map(|e| ParseError::from_compiler_error(e, &code)).collect::<Vec<_>>());
                return;
            }
        };
        stage += 1;
        print_progress(stage, pipeline);
        
        sleep(Duration::from_millis(100));

        let build_dir = Path::new("release");
        if !build_dir.exists() {
            fs::create_dir_all(build_dir).unwrap();
        }
        let project_name = get_project_name().unwrap_or_else(|| "output".to_string());

        let target_triple = target.unwrap_or_else(|| HOST.to_string());
        let obj_ext = if target_triple.contains("windows") { "obj" } else { "o" };
        let obj_path = build_dir.join(format!("{}.{}", project_name, obj_ext));

        if let Err(e) = emit_object_file(compiler.module, &target_triple, obj_path.to_str().unwrap()) {
            print_error("Obj Gen", &[ParseError::new(ErrorKind::Codegen, e, 0, 0, Some(code), None, Severity::Error)]);
            compiler.dispose();
            return;
        }
        compiler.dispose();
        stage += 1;
        print_progress(stage, pipeline);
        
        sleep(Duration::from_millis(100));

        let exe_ext = if target_triple.contains("windows") { ".exe" } else { "" };
        let exe_name = output_filename.unwrap_or_else(|| format!("{}{}", project_name, exe_ext));
        let exe_path = build_dir.join(&exe_name);

        stage += 1;
        print_progress(stage, pipeline);
        
        if let Err(e) = link_object_file(obj_path.to_str().unwrap(), exe_path.to_str().unwrap(), &target_triple) {
            print_error("Linking", &[ParseError::new(ErrorKind::Linker, e, 0, 0, None, None, Severity::Error)]);
            fs::remove_file(&obj_path).ok();
            return;
        }

        fs::remove_file(&obj_path).ok();

        println!();
        println!(
            "{} Build finished successfully! Binary at: {}",
            style("✔").green().bold(),
            style(exe_path.display()).yellow()
        );

    } else if cmd == "run" {
        stage += 1;
        print_progress(stage, pipeline);
        println!();

        match compile_and_run_jit(hir, mut_vars) {
            Ok(()) => {
                println!(
                    "\n{} Program executed successfully!",
                    style("✔").green().bold()
                );
            }
            Err(errors) => {
                print_error("JIT Execute", &errors.iter().map(|e| ParseError::from_compiler_error(e, &code)).collect::<Vec<_>>());
                return;
            }
        }
    }
    println!("💻 Community: {}/community", WEBSITE);
}

fn print_header() {
    let title = format!(
        "{} {} {}",
        style("Neuron — AxonScript Compiler").cyan().bold(),
        style("•").white().bold(),
        style("Build #0001 Pre-Alpha Demo").yellow().bold()
    );
    println!(
        "{line}\n{title}\n    💻 Community: {website}/community\n{line}",
        line = style("━").dim().to_string().repeat(60),
        title = title,
        website = WEBSITE
    );
}

fn print_progress(stage: usize, pipeline: &[&str]) {
    let bar_width: usize = 40;
    let progress_ratio = if !pipeline.is_empty() {
        stage as f32 / pipeline.len() as f32
    } else {
        0.0
    };
    let filled_width = (bar_width as f32 * progress_ratio).round() as usize;

    let bar = format!(
        "{}{}",
        "━".repeat(filled_width),
        "-".repeat(bar_width.saturating_sub(filled_width))
    );

    let stage_name = if stage > 0 && stage <= pipeline.len() {
        pipeline[stage - 1]
    } else if stage >= pipeline.len() {
        "Done"
    } else {
        "Starting..."
    };

    let line = format!(
        "PIPELINE [{}] {}",
        style(&bar).green(),
        style(stage_name).bold()
    );

    print!("\r\x1b[K{}", line);
    stdout().flush().unwrap();
}

fn print_error(phase: &str, errors: &[ParseError]) {
    println!();
    println!(
        "\n{} Stage: {}",
        style("✘").red().bold(),
        style(phase).red().bold()
    );

    for error in errors {
        println!("{} {}", style("").red().bold(), error.message);

        if let Some(src) = &error.src {
            if error.start < src.len() && error.end <= src.len() {
                let line_start = src[..error.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = src[error.start..]
                    .find('\n')
                    .map(|i| error.start + i)
                    .unwrap_or(src.len());
                if line_start < line_end {
                    let line_text = &src[line_start..line_end];
                    println!("{}", style(line_text).dim());
                    println!(
                        "{}",
                        " ".repeat(error.start - line_start) + &style("^").red().bold().to_string()
                    );
                }
            }
        }

        if let Some(suggestion) = &error.suggestion {
            println!(
                "{} {}",
                style("Hint:").cyan().bold(),
                style(suggestion).cyan()
            );
        }
    }

    println!(
        "\n📚 Docs: {}/docs\n🐞 Bugs: {}/bugs\n💻 Community: {}/community\n",
        WEBSITE, WEBSITE, WEBSITE
    );
}

fn print_warning(phase: &str, warnings: &[ParseError]) {
    println!();
    println!(
        "\n{} Stage: {}",
        style("!").yellow().bold(),
        style(phase).yellow().bold()
    );

    for warning in warnings {
        println!(
            "{} [{:?}] {}",
            style("").yellow().bold(),
            style(warning.kind).yellow(),
            warning.message
        );

        if let Some(src) = &warning.src {
             if warning.start < src.len() && warning.end <= src.len() {
                let line_start = src[..warning.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = src[warning.start..]
                    .find('\n')
                    .map(|i| warning.start + i)
                    .unwrap_or(src.len());
                if line_start < line_end {
                    let line_text = &src[line_start..line_end];
                    println!("{}", style(line_text).dim());
                    println!(
                        "{}",
                        " ".repeat(warning.start - line_start) + &style("^").yellow().bold().to_string()
                    );
                }
            }
        }

        if let Some(suggestion) = &warning.suggestion {
            println!(
                "{} {}",
                style("Hint:").cyan().bold(),
                style(suggestion).cyan()
            );
        }
    }
}

fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    stdout().flush().unwrap();
}

fn get_project_name() -> Option<String> {
    let path = Path::new("project.asml");
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(name) = line.trim().strip_prefix("_name_ = ") {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}