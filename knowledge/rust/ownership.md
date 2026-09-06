---
id: rust-ownership-basics
language: rust
category: ownership
title: Rust Ownership and Borrowing Basics
tags:
  - ownership
  - borrowing
  - references
  - borrow-checker
  - move-semantics
---
# Rust Ownership and Borrowing Basics

ทุกค่าใน Rust มี Owner (เจ้าของ) เพียงหนึ่งเดียวเสมอ เมื่อส่งค่าประเภท non-`Copy` จะเกิดการย้ายสิทธิ์ (Move) การใช้ References (`&` และ `&mut`) ช่วยให้เข้าถึงข้อมูลได้โดยไม่ต้องโอนย้าย ownership (Borrowing)

## ก่อนแก้ (Before - Move Issue)
```rust
fn print_len(s: String) {
    println!("{}", s.len());
}

let name = String::from("Rust");
print_len(name); // ownership ย้ายเข้าไปในฟังก์ชัน print_len แล้วถูก drop
println!("{name}"); // ERROR: borrow of moved value: `name`
```

## แก้แล้ว (After - Borrow Success)
```rust
fn print_len(s: &str) { // เปลี่ยนเป็นรับ reference แทน
    println!("{}", s.len());
}

let name = String::from("Rust");
print_len(&name); // ส่งแบบ borrow ด้วย &
println!("{name}"); // SUCCESS: name ยังคงใช้งานต่อได้ปกติ
```

## การทำงาน (How it works)
- **Ownership Rules**: แต่ละค่ามีเจ้าของคนเดียว เมื่อเจ้าของหลุด scope หน่วยความจำจะถูก clean up อัตโนมัติ
- **Borrowing Rules**: สามารถมี immutable reference (`&T`) หลายตัวพร้อมกันได้ หรือมี mutable reference (`&mut T`) ได้เพียงตัวเดียว
- ใช้ reference เสมอหากฟังก์ชันต้องการเพียงแค่อ่านหรือประมวลผลข้อมูลชั่วคราว
