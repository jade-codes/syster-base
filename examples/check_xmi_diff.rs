//! Check XMI roundtrip byte-level differences

use std::fs;
use syster::interchange::{ModelFormat, Xmi};

fn main() {
    let xmi = Xmi::default();
    let path = "/tmp/sysml-v2-release/sysml.library.xmi/Systems Library/Parts.sysmlx";

    let original = fs::read(path).unwrap();
    let model1 = xmi.read(&original).unwrap();
    let write1 = xmi.write(&model1).unwrap();
    let model2 = xmi.read(&write1).unwrap();
    let write2 = xmi.write(&model2).unwrap();

    println!("Original: {} bytes", original.len());
    println!("Write 1:  {} bytes", write1.len());
    println!("Write 2:  {} bytes", write2.len());
    println!("Write1 == Write2: {}", write1 == write2);

    if write1 != write2 {
        // Find first difference
        for (i, (b1, b2)) in write1.iter().zip(write2.iter()).enumerate() {
            if b1 != b2 {
                let start = if i > 100 { i - 100 } else { 0 };
                let end = std::cmp::min(i + 100, write1.len());
                println!("\nFirst diff at byte {}", i);
                println!("Context from write1:\n{}", String::from_utf8_lossy(&write1[start..end]));
                println!("\nContext from write2:\n{}", String::from_utf8_lossy(&write2[start..end]));
                break;
            }
        }
    } else {
        println!("\n✓ Output converged after first roundtrip!");
    }
}
