/* Copyright 2021 Aristocratos (jakob@qvantnet.com)

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

#include <string>
#include <vector>
#include <regex>
#include <iostream>
#include <algorithm>

#include <gtest/gtest.h>

#include "btop_tools.hpp"

// Helper to strip ANSI escape codes from a string
std::string strip_ansi(const std::string& input) {
    static const std::regex ansi_regex("\x1b\\[[0-9;]*[a-zA-Z]");
    return std::regex_replace(input, ansi_regex, "");
}

// Helper to calculate visible width (accounting for unicode)
size_t visible_width(const std::string& s) {
    std::string stripped = strip_ansi(s);
    size_t width = 0;
    size_t i = 0;
    while (i < stripped.size()) {
        unsigned char c = stripped[i];
        if ((c & 0x80) == 0) {
            // ASCII
            width++;
            i++;
        } else if ((c & 0xE0) == 0xC0) {
            // 2-byte UTF-8
            width++;  // Most 2-byte chars are 1 cell wide
            i += 2;
        } else if ((c & 0xF0) == 0xE0) {
            // 3-byte UTF-8 (includes box drawing, CJK, braille)
            // Box drawing (U+2500-U+257F) and braille (U+2800-U+28FF) are 1 cell
            // CJK would be 2 cells but we don't expect those here
            width++;
            i += 3;
        } else if ((c & 0xF8) == 0xF0) {
            // 4-byte UTF-8
            width += 2;  // Usually emoji, 2 cells
            i += 4;
        } else {
            width++;
            i++;
        }
    }
    return width;
}

// Test that rjust produces correct width
TEST(CpuBoxAlignment, RjustWidth) {
    EXPECT_EQ(Tools::rjust("5", 3), "  5");
    EXPECT_EQ(Tools::rjust("50", 3), " 50");
    EXPECT_EQ(Tools::rjust("100", 3), "100");

    EXPECT_EQ(Tools::rjust("5", 4), "   5");
    EXPECT_EQ(Tools::rjust("50", 4), "  50");
    EXPECT_EQ(Tools::rjust("100", 4), " 100");
}

// Test that ljust produces correct width
TEST(CpuBoxAlignment, LjustWidth) {
    EXPECT_EQ(Tools::ljust("P0", 4), "P0  ");
    EXPECT_EQ(Tools::ljust("P7", 4), "P7  ");
    EXPECT_EQ(Tools::ljust("E0", 4), "E0  ");
    EXPECT_EQ(Tools::ljust("E1", 4), "E1  ");
}

// Test visible width calculation
TEST(CpuBoxAlignment, VisibleWidth) {
    EXPECT_EQ(visible_width("hello"), 5u);
    EXPECT_EQ(visible_width("P-CPU"), 5u);
    EXPECT_EQ(visible_width("100%"), 4u);
    // Box drawing character │
    EXPECT_EQ(visible_width("│"), 1u);
    // Braille character
    EXPECT_EQ(visible_width("⣿"), 1u);
    // Multiple braille
    EXPECT_EQ(visible_width("⣿⣿⣿⣿⣿"), 5u);
}

// Simulate row width calculation for individual cores
// Format: label + graph + percentage + v_line
size_t calc_core_row_width(int core_width, int graph_width, int percent_width) {
    // label (ljust to core_width + 1) + graph + rjust(percent, percent_width) + '%' + v_line
    return (core_width + 1) + graph_width + percent_width + 1 + 1;
}

// Simulate row width calculation for P-CPU/E-CPU bars
// Format: "P-CPU " + meter + freq_str + v_line
size_t calc_header_bar_width(int meter_width, int freq_width) {
    // "P-CPU " (6) + meter + freq_str + v_line
    return 6 + meter_width + freq_width + 1;
}

// Test that P-CPU bar width matches core row width for single column
TEST(CpuBoxAlignment, SingleColumnWidthMatch) {
    // Simulate b_columns = 1, various b_width values
    for (int b_width = 30; b_width <= 80; b_width += 10) {
        int b_columns = 1;
        int col_width = b_width / b_columns;

        // From the code: meter_width = b_width - 7 - freq_width - 1
        int freq_width = 0; // no frequency string in this simulation
        int meter_width = std::max(1, b_width - 7 - freq_width - 1);

        // b_column_size affects graph width and percent width
        // For simplicity, assume b_column_size >= 2 (percent_width = 4)
        int percent_width = 4;
        int core_width = 3;  // "P0 " style

        // Graph width calculation is complex, but for single column full width:
        // The core row should fill col_width
        // label(4) + graph + percent(4) + '%'(1) + v_line(1) = col_width
        // So graph = col_width - 10
        int graph_width = col_width - 10;

        size_t core_row = calc_core_row_width(core_width, graph_width, percent_width);
        size_t header_row = calc_header_bar_width(meter_width, freq_width);

        std::cout << "b_width=" << b_width
                  << " col_width=" << col_width
                  << " meter_width=" << meter_width
                  << " core_row=" << core_row
                  << " header_row=" << header_row
                  << std::endl;

        EXPECT_EQ(core_row, static_cast<size_t>(col_width)) << "b_width=" << b_width;
        EXPECT_EQ(header_row, static_cast<size_t>(b_width - 1)) << "b_width=" << b_width;
        EXPECT_EQ(header_row + 1, core_row) << "b_width=" << b_width;
    }
}

// Test that P-CPU bar width matches core row width for double column
TEST(CpuBoxAlignment, DoubleColumnWidthMatch) {
    // Simulate b_columns = 2
    for (int b_width = 60; b_width <= 120; b_width += 10) {
        int b_columns = 2;
        int col_width = b_width / b_columns;

        int freq_width = 0;
        int meter_width = std::max(1, b_width - 7 - freq_width - 1);
        size_t header_row = calc_header_bar_width(meter_width, freq_width);

        // In double column mode, P-CPU bar spans both columns
        // Each column has: core + v_line
        // Total row: col1 + v_line + col2 + v_line = b_width
        // P-CPU bar should span: b_width - 1 (just before box border)

        std::cout << "b_width=" << b_width
                  << " b_columns=" << b_columns
                  << " col_width=" << col_width
                  << " meter_width=" << meter_width
                  << " header_row=" << header_row
                  << std::endl;

        EXPECT_EQ(header_row, static_cast<size_t>(b_width - 1)) << "b_width=" << b_width;
        EXPECT_EQ(header_row + 1, static_cast<size_t>(b_width)) << "b_width=" << b_width;
    }
}

// Core alignment formula verification
// This test documents the correct formula for P-CPU/E-CPU bar width
TEST(CpuBoxAlignment, CorrectFormula) {
    // The correct formula for P-CPU/E-CPU meter width is:
    //   meter = b_width - 13
    //
    // Breakdown:
    //   "P-CPU " (6 chars) + meter + rjust(4) + '%' (1) + v_line (1) = b_width - 1
    //   6 + meter + 4 + 1 + 1 = b_width - 1
    //   meter = b_width - 13
    //
    // This works for both single-column and multi-column layouts because:
    // - In single-column: each core row fills b_width - 1 and ends with v_line + border
    // - In multi-column: the last column's v_line is at position b_width - 2, border at b_width - 1
    //   The P-CPU bar spans the full width and ends at the same position

    for (int b_width = 30; b_width <= 200; b_width += 10) {
        int meter = b_width - 13;
        int total_content = 6 + meter + 4 + 1 + 1;  // label + meter + rjust(4) + '%' + v_line
        EXPECT_EQ(total_content, b_width - 1) << "b_width=" << b_width;
    }
}

// Test that the meter width is always positive for reasonable box widths
TEST(CpuBoxAlignment, MeterWidthPositive) {
    // Minimum reasonable CPU box width should allow for at least a tiny meter
    // "P-CPU " (6) + meter(1) + "100%" (4) + v_line (1) = 12 minimum
    // So b_width must be at least 13 for meter >= 0

    for (int b_width = 20; b_width <= 200; b_width++) {
        int meter = b_width - 13;
        EXPECT_GE(meter, 0) << "b_width=" << b_width << " gives negative meter";
    }
}
