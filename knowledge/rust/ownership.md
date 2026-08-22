---
id: rust-ownership-basics
language: rust
category: ownership
tags:
  - ownership
  - borrowing
  - references
  - borrow-checker
  - move-semantics
---
# Rust Ownership and Borrowing Basics

Every Rust value has an owner. Passing a non-`Copy` value by value transfers ownership. References let code access a value without taking ownership, while lifetime rules prevent references from outliving their data.

The borrow checker enforces that mutable access is exclusive and that moved values are not reused.
