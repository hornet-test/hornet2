//! 変換コマンドの実装
//!
//! Arazzoワークフローを各種テストスクリプト形式へ変換する。

use crate::converters::{ConvertOptions, Converter, K6Converter};
use crate::error::Result;
use crate::loader::{SourceDescriptionResolver, arazzo::load_arazzo};
use colored::Colorize;
use std::fs;
use std::path::Path;

/// `execute_convert` に渡す引数をまとめた構造体
pub struct ConvertCommandArgs<'a> {
    pub arazzo_path: &'a Path,
    pub output_path: Option<&'a Path>,
    pub target: &'a str,
    pub workflow_id: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub vus: Option<u32>,
    pub duration: Option<&'a str>,
    pub iterations: Option<u32>,
}

/// `execute_run` に渡す引数をまとめた構造体
pub struct RunCommandArgs<'a> {
    pub arazzo_path: &'a Path,
    pub engine: &'a str,
    pub workflow_id: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub vus: Option<u32>,
    pub duration: Option<&'a str>,
    pub iterations: Option<u32>,
}

/// 変換コマンドを実行する
pub fn execute_convert(args: ConvertCommandArgs<'_>) -> Result<()> {
    let ConvertCommandArgs {
        arazzo_path,
        output_path,
        target,
        workflow_id,
        base_url,
        vus,
        duration,
        iterations,
    } = args;

    // Arazzoファイルを読み込む
    let arazzo = load_arazzo(arazzo_path)?;
    eprintln!(
        "{} Loaded Arazzo file: {}",
        "✓".green(),
        arazzo_path.display()
    );

    // Load OpenAPI sources from sourceDescriptions
    let resolver_helper = SourceDescriptionResolver::new(arazzo_path)?;
    let source_result = resolver_helper.load_sources(&arazzo.source_descriptions);

    // Report source loading errors
    if !source_result.errors.is_empty() {
        eprintln!("{}", "⚠️  Some OpenAPI sources failed to load:".yellow());
        for err in &source_result.errors {
            eprintln!("  - Source '{}': {}", err.name, err.message);
        }
    }

    if source_result.resolver.get_all_specs().is_empty() {
        return Err(crate::error::HornetError::ValidationError(
            "No OpenAPI sources loaded successfully".to_string(),
        ));
    }

    eprintln!(
        "{} Loaded {} OpenAPI source(s)",
        "✓".green(),
        source_result.resolver.get_all_specs().len()
    );

    // 変換オプションを組み立てる
    let options = ConvertOptions {
        base_url: base_url.map(|s| s.to_string()),
        vus,
        duration: duration.map(|s| s.to_string()),
        iterations,
    };

    // ターゲットに応じてスクリプトを生成する
    let script = match target.to_lowercase().as_str() {
        "k6" => {
            let converter = K6Converter::new();

            if let Some(wf_id) = workflow_id {
                // 特定のワークフローのみ変換
                let workflow = arazzo
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == wf_id)
                    .ok_or_else(|| {
                        crate::error::HornetError::ValidationError(format!(
                            "Workflow '{}' not found",
                            wf_id
                        ))
                    })?;

                converter.convert_workflow(workflow, &source_result.resolver, &options)?
            } else {
                // 全ワークフローを変換
                converter.convert_spec(&arazzo, &source_result.resolver, &options)?
            }
        }
        _ => {
            return Err(crate::error::HornetError::ValidationError(format!(
                "Unsupported target format: {}. Supported: k6",
                target
            )));
        }
    };

    // 生成結果を出力する
    if let Some(path) = output_path {
        fs::write(path, &script)?;
        println!(
            "{} Generated {} script: {}",
            "✓".green(),
            target,
            path.display()
        );
    } else {
        println!("\n{}", script);
    }

    Ok(())
}

/// 変換と実行をまとめて行う `run` コマンドを実行する
pub fn execute_run(args: RunCommandArgs<'_>) -> Result<()> {
    let RunCommandArgs {
        arazzo_path,
        engine,
        workflow_id,
        base_url,
        vus,
        duration,
        iterations,
    } = args;

    use crate::runner::{K6Runner, Runner};

    // まずスクリプトを生成する
    println!("{} Generating test script...", "→".blue());

    // 入力ファイルを読み込む
    let arazzo = load_arazzo(arazzo_path)?;

    // Load OpenAPI sources from sourceDescriptions
    let resolver_helper = SourceDescriptionResolver::new(arazzo_path)?;
    let source_result = resolver_helper.load_sources(&arazzo.source_descriptions);

    if !source_result.errors.is_empty() {
        eprintln!("{}", "⚠️  Some OpenAPI sources failed to load:".yellow());
        for err in &source_result.errors {
            eprintln!("  - Source '{}': {}", err.name, err.message);
        }
    }

    if source_result.resolver.get_all_specs().is_empty() {
        return Err(crate::error::HornetError::ValidationError(
            "No OpenAPI sources loaded successfully".to_string(),
        ));
    }

    let options = ConvertOptions {
        base_url: base_url.map(|s| s.to_string()),
        vus,
        duration: duration.map(|s| s.to_string()),
        iterations,
    };

    match engine.to_lowercase().as_str() {
        "k6" => {
            let converter = K6Converter::new();
            let runner = K6Runner::new();

            // k6が利用可能か確認する
            if !runner.is_available() {
                return Err(crate::error::HornetError::ValidationError(
                    "k6 is not installed or not in PATH. Please install k6 first: https://k6.io/docs/get-started/installation/"
                        .to_string(),
                ));
            }

            println!("{} k6 version: {}", "✓".green(), runner.version()?);

            let script = if let Some(wf_id) = workflow_id {
                let workflow = arazzo
                    .workflows
                    .iter()
                    .find(|w| w.workflow_id == wf_id)
                    .ok_or_else(|| {
                        crate::error::HornetError::ValidationError(format!(
                            "Workflow '{}' not found",
                            wf_id
                        ))
                    })?;
                converter.convert_workflow(workflow, &source_result.resolver, &options)?
            } else {
                // ワークフロー未指定時は先頭を使用
                converter.convert_workflow(
                    &arazzo.workflows[0],
                    &source_result.resolver,
                    &options,
                )?
            };

            println!("{} Running tests with k6...\n", "→".blue());

            let result = runner.run_script_content(&script)?;

            // 実行結果の標準出力・標準エラーを表示
            if !result.stdout.is_empty() {
                println!("{}", result.stdout);
            }

            if !result.stderr.is_empty() {
                eprintln!("{}", result.stderr);
            }

            // サマリーを表示
            if result.success {
                println!("\n{} Test run completed successfully!", "✓".green());
            } else {
                println!(
                    "\n{} Test run failed with exit code: {}",
                    "✗".red(),
                    result.exit_code
                );
            }

            if let Some(metrics) = result.metrics {
                println!("\n{}", "Metrics Summary:".bold());
                println!("  HTTP Requests: {}", metrics.http_reqs);
                println!("  Iterations: {}", metrics.iterations);
                println!("  Avg Response Time: {:.2}ms", metrics.avg_response_time_ms);
                println!(
                    "  Checks: {} passed, {} failed",
                    metrics.checks_passed, metrics.checks_failed
                );
            }

            if !result.success {
                return Err(crate::error::HornetError::ValidationError(format!(
                    "Test run failed with exit code: {}",
                    result.exit_code
                )));
            }
        }
        _ => {
            return Err(crate::error::HornetError::ValidationError(format!(
                "Unsupported engine: {}. Supported: k6",
                engine
            )));
        }
    }

    Ok(())
}
