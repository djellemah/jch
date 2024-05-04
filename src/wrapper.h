#pragma once

#include "rust/cxx.h"

struct RustStream;
struct RustHandler;

void parse(RustHandler & handler, RustStream & incoming);
void from_file(rust::String filename, RustHandler & handler);
