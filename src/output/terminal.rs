use std::collections::BTreeMap;

use crate::{config::LoadedConfig, knowledge::SearchResult, scanner::ScanReport};

pub fn print_config(loaded: &LoadedConfig) {
    println!("libraryCube configuration\n");
    println!("Config file");
    if loaded.found {
        println!("  {}\n", loaded.path.display());
    } else {
        println!("  not found ({})\n", loaded.path.display());
        println!("Using default configuration.\n");
    }
    println!("Output");
    println!("  Language:        {}\n", loaded.config.output.language);
    println!("Scanner");
    println!(
        "  Max file size:   {} KB",
        loaded.config.scanner.max_file_size_kb
    );
    println!(
        "  Ignore hidden:   {}\n",
        if loaded.config.scanner.ignore_hidden {
            "yes"
        } else {
            "no"
        }
    );
    println!("Memory");
    println!("  Mode:            {}", loaded.config.memory.mode);
    println!(
        "  History:         {}",
        crate::history::history_path().display()
    );
    println!(
        "  Persistence:     {}",
        if loaded.config.memory.mode == "persistent" {
            "explicitly enabled; redacted bounded history is restored"
        } else {
            "disabled; chat context exists only in the active process"
        }
    );
    println!("\nAI");
    match loaded.config.ai.provider.as_str() {
        "off" => println!("  Provider:        off (deterministic mode)"),
        provider => {
            println!("  Provider:        {provider}");
            println!("  Model:           {}", loaded.config.ai.model);
            if provider == "openai-compat" {
                println!("  Base URL:        {}", loaded.config.ai.base_url);
            }
        }
    }
}

pub fn print_scan(report: &ScanReport, show_tree: bool) {
    println!("Project detected\n");
    print_values("Language", &report.project.languages);
    print_values("Build system", &report.project.build_systems);
    print_values("Containers", &report.project.containers);
    print_values("Frameworks", &report.project.frameworks);
    if !report.project.source_directories.is_empty() {
        println!("Source directories");
        for path in &report.project.source_directories {
            println!("  {}/", path.display());
        }
        println!();
    }
    print_values("Additional", &report.project.additional);

    if show_tree {
        print_tree(report);
    }

    println!("Scan Summary\n");
    println!("Project root\n  {}\n", report.project.root.display());
    println!(
        "Eligible files inventoried\n  {}\n",
        report.files_in_inventory
    );
    println!("File contents read\n  {}\n", report.file_contents_read);
    println!("Rust source files\n  {}\n", report.rust_source_files);
    println!("Configuration files\n  {}\n", report.configuration_files);
    println!("Knowledge documents\n  {}\n", report.knowledge_documents);
    if report.large_files_skipped > 0 {
        println!("Large files skipped\n  {}\n", report.large_files_skipped);
    }
    if !report.ignored_directories.is_empty() {
        println!("Ignored directories");
        for directory in &report.ignored_directories {
            println!("  {directory}");
        }
    }
    if !report.warnings.is_empty() {
        println!("\n! Scan warnings");
        for warning in &report.warnings {
            println!("  {warning}");
        }
    }
}

fn print_values(label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("{label}");
    for value in values {
        println!("  {value}");
    }
    println!();
}

#[derive(Default)]
struct TreeNode {
    directory: bool,
    children: BTreeMap<String, TreeNode>,
}

fn print_tree(report: &ScanReport) {
    let mut root = TreeNode {
        directory: true,
        ..TreeNode::default()
    };
    for entry in &report.entries {
        let components: Vec<_> = entry
            .path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        insert_tree_entry(&mut root, &components, entry.is_directory);
    }
    let name = report
        .project
        .root
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| report.project.root.as_os_str().to_string_lossy());
    println!("Project Tree\n\n{name}");
    print_tree_children(&root, "");
    println!();
}

fn insert_tree_entry(node: &mut TreeNode, components: &[String], directory: bool) {
    let Some((component, rest)) = components.split_first() else {
        return;
    };
    let child = node.children.entry(component.clone()).or_default();
    if rest.is_empty() {
        child.directory = directory;
    } else {
        child.directory = true;
        insert_tree_entry(child, rest, directory);
    }
}

fn print_tree_children(node: &TreeNode, prefix: &str) {
    let count = node.children.len();
    for (index, (name, child)) in node.children.iter().enumerate() {
        let last = index + 1 == count;
        let branch = if last { "└──" } else { "├──" };
        let suffix = if child.directory { "/" } else { "" };
        println!("{prefix}{branch} {name}{suffix}");
        let child_prefix = format!("{prefix}{}   ", if last { " " } else { "│" });
        print_tree_children(child, &child_prefix);
    }
}

pub fn print_search(query: &str, results: &[SearchResult]) {
    print_search_language(query, results, "en");
}

pub fn print_search_language(query: &str, results: &[SearchResult], language: &str) {
    if language == "th" {
        println!("ค้นหาคลังความรู้\n");
        println!("คำค้น\n  {query}\n");
        println!("ผลลัพธ์");
    } else {
        println!("Knowledge Search\n");
        println!("Query\n  {query}\n");
        println!("Results");
    }
    if results.is_empty() {
        println!(
            "\n  {}",
            if language == "th" {
                "ไม่พบความรู้ในเครื่องที่ตรงกับคำค้นนี้"
            } else {
                "No local knowledge matched this query."
            }
        );
        return;
    }
    for (index, result) in results.iter().take(10).enumerate() {
        println!("\n{}. {}", index + 1, result.document.title);
        if language == "th" {
            println!("   เหตุผลที่ตรงกัน: {}", result.match_reason);
            println!("   แหล่งอ้างอิง: {}", result.document.source_id);
            println!("   ตำแหน่ง: {}", result.document.path);
            println!("   เนื้อหาที่เกี่ยวข้อง: {}", result.excerpt);
        } else {
            println!("   Match: {}", result.match_reason);
            println!("   Source: {}", result.document.source_id);
            println!("   Locator: {}", result.document.path);
            println!("   Excerpt: {}", result.excerpt);
        }
    }
}
