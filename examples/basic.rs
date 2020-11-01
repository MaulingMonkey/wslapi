use wslapi::*;



fn main() {
    let ubuntu = "Ubuntu";
    let nonexistant = "Nonexistant";

    let wsl = Library::new().unwrap();
    assert!( wsl.is_distribution_registered(ubuntu));
    assert!(!wsl.is_distribution_registered(nonexistant));
    assert!(wsl.get_distribution_configuration(nonexistant).is_err());

    let cfg = wsl.get_distribution_configuration(ubuntu).unwrap();
    assert!(cfg.default_uid == 0 || (1000 ..= 2000).contains(&cfg.default_uid)); // Root or regular UID
    assert!(cfg.flags & WSL_DISTRIBUTION_FLAGS::DEFAULT == WSL_DISTRIBUTION_FLAGS::DEFAULT);
    assert!((1..2).contains(&cfg.version)); // WSL version

    wsl.launch_interactive(ubuntu, "echo testing 123", true).unwrap();

    let stdin  = "echo testing 456\necho PATH: ${PATH}\nasdf";
    let stdout = std::fs::File::create("target/basic.txt").unwrap();
    let stderr = std::fs::File::create("CON").unwrap();
    wsl.launch(ubuntu, "sh", true, stdin, stdout, stderr).unwrap().wait().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
}
