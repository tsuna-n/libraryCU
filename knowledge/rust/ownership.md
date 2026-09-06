---
id: rust-ownership-basics
kind: concept
language: rust
category: ownership
title: Rust Ownership and Borrowing Basics
title_th: พื้นฐาน Ownership และ Borrowing ใน Rust
tags:
  - ownership
  - borrowing
  - references
  - borrow-checker
  - move-semantics
keywords:
  - owner
  - move
  - reference
  - เจ้าของข้อมูล
  - การยืม
---
<!-- lbc:en -->
# Rust Ownership and Borrowing Basics

## Quick answer

Every Rust value has one owner. Passing a non-`Copy` value transfers ownership; pass `&T` to read without taking ownership and `&mut T` to modify through an exclusive borrow.

## Example

```rust
fn print_len(s: &str) {
    println!("{}", s.len());
}

let name = String::from("Rust");
print_len(&name);
println!("{name}");
```

## Rules to remember

- Any number of shared references (`&T`), or one mutable reference (`&mut T`).
- A value is dropped when its owner leaves scope.

<!-- lbc:th -->
# พื้นฐาน Ownership และ Borrowing ใน Rust

## คำตอบสั้น ๆ

ค่าทุกค่าใน Rust มีเจ้าของหนึ่งคน การส่งค่าชนิดที่ไม่ใช่ `Copy` จะย้าย ownership ให้ใช้ `&T` เมื่อต้องการอ่านโดยไม่รับ ownership และใช้ `&mut T` เมื่อต้องการแก้ไขผ่านการยืมแบบเฉพาะผู้เดียว

## ตัวอย่าง

```rust
fn print_len(s: &str) {
    println!("{}", s.len());
}

let name = String::from("Rust");
print_len(&name);
println!("{name}");
```

## กฎที่ควรจำ

- มี shared reference (`&T`) ได้หลายตัว หรือ mutable reference (`&mut T`) ได้หนึ่งตัว
- ค่าจะถูก drop เมื่อเจ้าของหลุดออกจาก scope
