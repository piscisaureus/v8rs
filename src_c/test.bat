@echo off
cls && clang-cl -std:c++17 /LD lib.cpp && cl -std:c++17 /LD /EHsc lib.cpp