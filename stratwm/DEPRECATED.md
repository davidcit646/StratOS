# stratwm — DEPRECATED / ARCHIVED

This crate is archived. Do not implement or extend.

## Why

stratwm was created as a Custom First Rust compositor attempt.
After review, this approach (Rust wlroots FFI via raw pointers) adds
complexity without gain — wlroots dependency remains either way.

## Current compositor

The active compositor is `stratvm` (C/wlroots). It is the correct
solution for now and is referred to everywhere as **stratvm**.

## Future

A true Custom First compositor (direct DRM/KMS, no wlroots) is
planned as a late-phase replacement for stratvm. That work will
happen here when the DE is otherwise complete.

## Status

ARCHIVED — 2026-04-16
