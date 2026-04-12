fn main() {
    let names = vox_clavis::managed_secret_env_names();
    println!("{:#?}", names);
}
