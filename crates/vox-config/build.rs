fn main() {
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-pins.v1.yaml");
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-routing.v1.yaml");
}
