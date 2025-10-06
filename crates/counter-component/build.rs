fn main() {
    println!("cargo:rerun-if-changed=../../wit/vello/canvas.wit");
    println!("cargo:rerun-if-changed=src/bindings/canvas_app.rs");
}
