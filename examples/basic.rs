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
    assert!((1..=2).contains(&cfg.version)); // WSL version

    println!("Environment varibales:");
    for (k,v) in cfg.default_environment_variables.iter() {
        // XXX: This might be inaccurate / depend on the linux locale?  Good enough for a demo though!
        let k = String::from_utf8_lossy(k);
        let v = String::from_utf8_lossy(v);
        println!("{: <10} = {}", k, v);
    }

    wsl.launch_interactive(ubuntu, "echo testing 123", true).unwrap();

    let stdin  = "echo testing 456\necho PATH: ${PATH}\nasdf";
    let stdout = std::fs::File::create("target/basic.txt").unwrap();
    let stderr = std::fs::File::create("CON").unwrap();
    wsl.launch(ubuntu, "sh", true, stdin, stdout, stderr).unwrap().wait().unwrap();

    println!("\rPress ENTER to quit");
    let _ = std::io::stdin().read_line(&mut String::new());
}
