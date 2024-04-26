#pragma once

#include "rust/cxx.h"

class RustStream;
class RustHandler;

void parse(RustHandler & handler, RustStream & incoming);
