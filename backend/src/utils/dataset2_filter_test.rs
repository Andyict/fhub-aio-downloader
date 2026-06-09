#[cfg(test)]
mod tests {
    use crate::utils::parser::FilenameParser;
    use crate::utils::smart_tokenizer::smart_parse;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    #[derive(serde::Deserialize)]
    struct DatasetItem {
        name: String,
    }

    #[derive(serde::Deserialize)]
    struct Dataset {
        data: Vec<DatasetItem>,
    }

    #[test]
    fn count_dataset2_parsing_success() {
        let file_path = std::env::var("FHUB_DATASET2_FIXTURE")
            .unwrap_or_else(|_| "debug_log/dataset_2.json".to_string());

        if !Path::new(&file_path).exists() {
            eprintln!("Skipping dataset2 parsing comparison; fixture not found at {}", file_path);
            return;
        }

        let file = File::open(&file_path).expect("Failed to open dataset file");
        let reader = BufReader::new(file);
        let dataset: Dataset = serde_json::from_reader(reader).expect("Failed to parse JSON");

        let mut old_passed = 0;
        let mut new_passed = 0;
        let mut old_failed_examples = Vec::new();
        let mut new_failed_examples = Vec::new();

        println!("\n📊 Dataset 2 Filename Parsing Filter Results\n");
        println!("Total files: {}\n", dataset.data.len());

        for item in &dataset.data {
            let filename = &item.name;
            
            // Old method
            let old = FilenameParser::parse(filename);
            let old_has_episode = old.episode.is_some();
            
            // New method
            let new = smart_parse(filename);
            let new_has_episode = new.episode.is_some();

            if old_has_episode {
                old_passed += 1;
            } else if old_failed_examples.len() < 5 {
                old_failed_examples.push(filename.clone());
            }

            if new_has_episode {
                new_passed += 1;
            } else if new_failed_examples.len() < 5 {
                new_failed_examples.push(filename.clone());
            }
        }

        println!("┌─────────────────────────────────────────────────────┐");
        println!("│           FILENAME PARSING FILTER RESULTS           │");
        println!("├─────────────────────────────────────────────────────┤");
        println!("│ Method              │ Passed  │ Failed  │ Pass Rate │");
        println!("├─────────────────────┼─────────┼─────────┼───────────┤");
        println!("│ Old (FilenameParser)│ {:>7} │ {:>7} │   {:>5.1}%  │", 
                 old_passed, 
                 dataset.data.len() - old_passed,
                 (old_passed as f64 / dataset.data.len() as f64) * 100.0);
        println!("│ New (SmartTokenizer)│ {:>7} │ {:>7} │   {:>5.1}%  │", 
                 new_passed,
                 dataset.data.len() - new_passed,
                 (new_passed as f64 / dataset.data.len() as f64) * 100.0);
        println!("└─────────────────────┴─────────┴─────────┴───────────┘");

        println!("\n📈 Improvement: +{} files ({:.1}% increase)",
                 new_passed - old_passed,
                 ((new_passed - old_passed) as f64 / old_passed as f64) * 100.0);

        if !old_failed_examples.is_empty() {
            println!("\n❌ Old Method Failed Examples (first 5):");
            for (i, example) in old_failed_examples.iter().enumerate() {
                println!("   {}. {}", i + 1, example);
            }
        }

        if !new_failed_examples.is_empty() {
            println!("\n❌ New Method Failed Examples (first 5):");
            for (i, example) in new_failed_examples.iter().enumerate() {
                println!("   {}. {}", i + 1, example);
            }
        }

        println!("\n✅ Conclusion:");
        println!("   Old Method: {} files would be available for smart search", old_passed);
        println!("   New Method: {} files would be available for smart search", new_passed);
        println!("   Difference: {} more files accessible with new method\n", new_passed - old_passed);
    }
}
