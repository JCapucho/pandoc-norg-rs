use std::fs;
use std::process::{Command, Stdio};

#[test]
fn convert() {
    let root = env!("CARGO_MANIFEST_DIR");
    let pandoc_exists = Command::new("pandoc").spawn().is_ok();

    for entry in fs::read_dir(format!("{root}/tests/in")).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().into_string().unwrap();

        println!("Processing {file_name}");

        let content = fs::read_to_string(entry.path()).expect("Couldn't read test file");

        let mut frontend = pandoc_norg_converter::Frontend::default();
        let document = frontend.convert(&content);

        let json_out = fs::File::create(format!("{root}/tests/out/{file_name}.json"))
            .expect("Failed to create output file");
        serde_json::to_writer_pretty(json_out, &document).expect("Failed to output json");

        if pandoc_exists {
            let out = format!("{root}/tests/out/{file_name}.md");
            let mut child = Command::new("pandoc")
                .args(["-f", "json", "-o", &out])
                .stdin(Stdio::piped())
                .spawn()
                .expect("Failed to spawn pandoc");

            let stdin = child.stdin.take().expect("Failed to open stdin");
            serde_json::to_writer(stdin, &document).expect("Failed to pipe json");

            assert!(child.wait().expect("command wasn't running").success());
        }
    }

    assert!(pandoc_exists, "Tests require the pandoc executable");
}
