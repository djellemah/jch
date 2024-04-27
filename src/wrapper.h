#pragma once

#include "rust/cxx.h"

class RustStream;
class RustHandler;

void parse(RustHandler & handler, RustStream & incoming);
void from_file(rust::String filename, RustHandler & handler);
