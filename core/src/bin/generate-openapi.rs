//! # OpenAPI 规范生成器
//!
//! 从代码自动生成 OpenAPI 规范文档。
//!
//! 用法：
//! ```bash
//! # 生成 JSON 格式
//! cargo run --bin generate-openapi -- --format json
//!
//! # 生成 YAML 格式
//! cargo run --bin generate-openapi -- --format yaml
//!
//! # 输出到指定目录
//! cargo run --bin generate-openapi -- --output ../docs/api/data-plane/
//! ```

use std::fs;
use std::path::PathBuf;
use clap::Parser;
use veridactus_core::http::openapi::VeridactusDataPlaneApi;

#[derive(Debug, Parser)]
struct Args {
    /// 输出格式：json 或 yaml
    #[arg(short, long, default_value = "json")]
    format: String,

    /// 输出目录
    #[arg(short, long, default_value = "openapi")]
    output: String,

    /// 输出文件名
    #[arg(short, long, default_value = "data-plane-v0.2.1")]
    filename: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args = Args::parse();

    // 生成 OpenAPI 规范
    let openapi = VeridactusDataPlaneApi::openapi();

    // 创建输出目录
    let output_dir = PathBuf::from(&args.output);
    fs::create_dir_all(&output_dir)?;

    // 根据格式生成文件
    let version = "0.2.1";
    match args.format.as_str() {
        "json" => {
            let filename = format!("{}-{}.json", args.filename, version);
            let path = output_dir.join(&filename);
            let json = serde_json::to_string_pretty(&openapi)?;
            fs::write(&path, json)?;
            println!("✅ Generated: {}", path.display());
        }
        "yaml" => {
            let filename = format!("{}-{}.yaml", args.filename, version);
            let path = output_dir.join(&filename);
            let yaml = serde_yaml::to_string(&openapi)?;
            fs::write(&path, yaml)?;
            println!("✅ Generated: {}", path.display());
        }
        "both" => {
            // 生成 JSON
            let json_filename = format!("{}-{}.json", args.filename, version);
            let json_path = output_dir.join(&json_filename);
            let json = serde_json::to_string_pretty(&openapi)?;
            fs::write(&json_path, json)?;
            println!("✅ Generated: {}", json_path.display());

            // 生成 YAML
            let yaml_filename = format!("{}-{}.yaml", args.filename, version);
            let yaml_path = output_dir.join(&yaml_filename);
            let yaml = serde_yaml::to_string(&openapi)?;
            fs::write(&yaml_path, yaml)?;
            println!("✅ Generated: {}", yaml_path.display());
        }
        _ => {
            return Err(format!(
                "Unsupported format: {}. Use 'json', 'yaml', or 'both'.",
                args.format
            )
            .into());
        }
    }

    println!("\n📄 OpenAPI spec version: {}", version);
    println!("🔗 Specification: https://spec.openapis.org/oas/v3.0.3");

    Ok(())
}
