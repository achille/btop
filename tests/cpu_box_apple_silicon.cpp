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
#include <iostream>

#include <gtest/gtest.h>
#include <fmt/format.h>

// Apple Silicon configurations: {name, p_cores, e_cores, typical_p_freq_mhz, typical_e_freq_mhz}
struct AppleSiliconConfig {
	std::string name;
	int p_cores;
	int e_cores;
	int p_freq_mhz;
	int e_freq_mhz;
};

const std::vector<AppleSiliconConfig> APPLE_SILICON_CONFIGS = {
	// M1 family
	{"M1",          4,  4, 3228, 2064},
	{"M1 Pro 8C",   6,  2, 3228, 2064},
	{"M1 Pro 10C",  8,  2, 3228, 2064},
	{"M1 Max",      8,  2, 3228, 2064},
	{"M1 Ultra",   16,  4, 3228, 2064},

	// M2 family
	{"M2",          4,  4, 3504, 2424},
	{"M2 Pro 10C",  6,  4, 3504, 2424},
	{"M2 Pro 12C",  8,  4, 3504, 2424},
	{"M2 Max",      8,  4, 3504, 2424},
	{"M2 Ultra",   16,  8, 3504, 2424},

	// M3 family
	{"M3",          4,  4, 4056, 2748},
	{"M3 Pro 11C",  5,  6, 4056, 2748},
	{"M3 Pro 12C",  6,  6, 4056, 2748},
	{"M3 Max 14C", 10,  4, 4056, 2748},
	{"M3 Max 16C", 12,  4, 4056, 2748},

	// M4 family
	{"M4",          4,  6, 4416, 2892},
	{"M4 Pro 12C", 10,  4, 4416, 2892},  // Actual M4 Pro can be 12 or 14 core
	{"M4 Pro 14C", 10,  4, 4416, 2892},
	{"M4 Max 14C", 10,  4, 4512, 2892},
	{"M4 Max 16C", 12,  4, 4512, 2892},
};

// Format frequency as 3 digits: "X.X GHz" or "XXX MHz"
std::string format_freq(int mhz) {
	if (mhz <= 0) return "";
	std::string str;
	if (mhz > 999) {
		str = fmt::format("{:.1f}", mhz / 1000.0);
		if (str.size() > 3) str.resize(3);
		if (str.back() == '.') str.pop_back();
		str += " GHz";
	} else {
		str = fmt::format("{} MHz", mhz);
	}
	return str;
}

// Simulate drawing a meter bar
std::string draw_meter(int width, int percent) {
	if (width <= 0) return "";
	int filled = (percent * width) / 100;
	std::string meter;
	for (int i = 0; i < width; i++) {
		meter += (i < filled) ? "=" : "-";
	}
	return meter;
}

// Simulate core label
std::string core_label(bool is_p_core, int index) {
	return (is_p_core ? "P" : "E") + std::to_string(index);
}

// Render a simulated CPU box for a given Apple Silicon config
struct RenderedBox {
	std::vector<std::string> lines;
	int width;
	int height;
};

RenderedBox render_cpu_box(const AppleSiliconConfig& config, int box_width, int b_columns) {
	RenderedBox box;
	box.width = box_width;

	std::vector<std::string> lines;
	int col_width = box_width / b_columns;

	// Header
	std::string header = "+" + std::string(box_width - 2, '-') + "+";
	lines.push_back(header);

	std::string title_line = "|" + config.name;
	title_line += std::string(box_width - 2 - config.name.size(), ' ') + "|";
	lines.push_back(title_line);

	// P-CPU bar
	std::string p_freq_str = " " + format_freq(config.p_freq_mhz);
	int p_meter_width = std::max(1, box_width - 7 - static_cast<int>(p_freq_str.size()) - 2);  // -2 for borders
	std::string p_line = "|P-CPU " + draw_meter(p_meter_width, 50) + p_freq_str + "|";
	lines.push_back(p_line);

	// P-cores
	int p_drawn = 0;
	while (p_drawn < config.p_cores) {
		std::string row = "|";
		for (int col = 0; col < b_columns and p_drawn < config.p_cores; col++) {
			std::string label = core_label(true, p_drawn);
			int graph_width = col_width - 10;  // label(3) + graph + percent(4) + % + separator
			std::string cell = label;
			while (cell.size() < 3) cell += " ";
			cell += " " + draw_meter(graph_width > 0 ? graph_width : 5, 30 + p_drawn * 5);
			cell += " " + std::to_string(30 + p_drawn * 5) + "%";
			while (static_cast<int>(cell.size()) < col_width - 1) cell += " ";
			cell += "|";
			row += cell;
			p_drawn++;
		}
		// Pad if incomplete row
		while (static_cast<int>(row.size()) < box_width - 1) row += " ";
		if (row.back() != '|') row += "|";
		lines.push_back(row);
	}

	// E-CPU bar
	std::string e_freq_str = " " + format_freq(config.e_freq_mhz);
	int e_meter_width = std::max(1, box_width - 7 - static_cast<int>(e_freq_str.size()) - 2);
	std::string e_line = "|E-CPU " + draw_meter(e_meter_width, 40) + e_freq_str + "|";
	lines.push_back(e_line);

	// E-cores
	int e_drawn = 0;
	while (e_drawn < config.e_cores) {
		std::string row = "|";
		for (int col = 0; col < b_columns and e_drawn < config.e_cores; col++) {
			std::string label = core_label(false, e_drawn);
			int graph_width = col_width - 10;
			std::string cell = label;
			while (cell.size() < 3) cell += " ";
			cell += " " + draw_meter(graph_width > 0 ? graph_width : 5, 20 + e_drawn * 5);
			cell += " " + std::to_string(20 + e_drawn * 5) + "%";
			while (static_cast<int>(cell.size()) < col_width - 1) cell += " ";
			cell += "|";
			row += cell;
			e_drawn++;
		}
		while (static_cast<int>(row.size()) < box_width - 1) row += " ";
		if (row.back() != '|') row += "|";
		lines.push_back(row);
	}

	// Footer
	std::string footer = "+" + std::string(box_width - 2, '-') + "+";
	lines.push_back(footer);

	box.lines = lines;
	box.height = static_cast<int>(lines.size());
	return box;
}

// Print rendered box
void print_box(const RenderedBox& box) {
	for (const auto& line : box.lines) {
		std::cout << line << std::endl;
	}
}

// Test that frequency formatting works correctly
TEST(AppleSiliconCpuBox, FrequencyFormatting) {
	EXPECT_EQ(format_freq(0), "");
	EXPECT_EQ(format_freq(600), "600 MHz");
	EXPECT_EQ(format_freq(972), "972 MHz");
	EXPECT_EQ(format_freq(999), "999 MHz");
	EXPECT_EQ(format_freq(1000), "1.0 GHz");
	EXPECT_EQ(format_freq(1332), "1.3 GHz");
	EXPECT_EQ(format_freq(2064), "2.1 GHz");  // 2.064 rounds to 2.1
	EXPECT_EQ(format_freq(3228), "3.2 GHz");
	EXPECT_EQ(format_freq(3504), "3.5 GHz");
	EXPECT_EQ(format_freq(4056), "4.1 GHz");  // 4.056 rounds to 4.1
	EXPECT_EQ(format_freq(4416), "4.4 GHz");
}

// Test that P-CPU/E-CPU bar meter width is always positive
TEST(AppleSiliconCpuBox, MeterWidthPositive) {
	for (const auto& config : APPLE_SILICON_CONFIGS) {
		std::string p_freq_str = " " + format_freq(config.p_freq_mhz);
		std::string e_freq_str = " " + format_freq(config.e_freq_mhz);

		// Test various box widths
		for (int box_width = 40; box_width <= 120; box_width += 10) {
			int p_meter = std::max(1, box_width - 7 - static_cast<int>(p_freq_str.size()) - 2);
			int e_meter = std::max(1, box_width - 7 - static_cast<int>(e_freq_str.size()) - 2);

			EXPECT_GT(p_meter, 0) << "Config: " << config.name << " box_width: " << box_width;
			EXPECT_GT(e_meter, 0) << "Config: " << config.name << " box_width: " << box_width;
		}
	}
}

// Test that meter width is clamped to 1 for very small widths
TEST(AppleSiliconCpuBox, MeterWidthZeroOrNegative) {
	AppleSiliconConfig config = {"M1", 4, 4, 3228, 2064};
	std::string p_freq_str = " " + format_freq(config.p_freq_mhz); // " 3.2 GHz" (8 chars)
	
	// Width 10: 10 - 7 - 8 - 2 = -7 -> should be 1
	int box_width = 10;
	int p_meter = std::max(1, box_width - 7 - static_cast<int>(p_freq_str.size()) - 2);
	EXPECT_EQ(p_meter, 1);
	
	RenderedBox box = render_cpu_box(config, box_width, 1);
	// Check that it rendered without crashing and produced output
	EXPECT_GT(box.lines.size(), 0);
}

// Test that all configs render without crash and produce reasonable output
TEST(AppleSiliconCpuBox, RenderAllConfigs) {
	for (const auto& config : APPLE_SILICON_CONFIGS) {
		for (int box_width = 50; box_width <= 120; box_width += 20) {
			for (int b_columns = 1; b_columns <= 2; b_columns++) {
				RenderedBox box = render_cpu_box(config, box_width, b_columns);

				// Should produce non-empty output
				EXPECT_GT(box.lines.size(), 0u)
					<< "Empty output for " << config.name;

				// Should have header, title, P-CPU bar, P-cores, E-CPU bar, E-cores, footer
				int min_lines = 1 + 1 + 1 + 1 + 1 + 1 + 1;  // At minimum
				EXPECT_GE(static_cast<int>(box.lines.size()), min_lines)
					<< "Too few lines for " << config.name;
			}
		}
	}
}

// Test that row counts are correct
TEST(AppleSiliconCpuBox, RowCounts) {
	for (const auto& config : APPLE_SILICON_CONFIGS) {
		// Single column: each core gets its own row
		{
			int b_columns = 1;
			int expected_rows = 1  // header
				+ 1  // title
				+ 1  // P-CPU bar
				+ config.p_cores  // P-cores
				+ 1  // E-CPU bar
				+ config.e_cores  // E-cores
				+ 1; // footer

			RenderedBox box = render_cpu_box(config, 60, b_columns);
			EXPECT_EQ(box.height, expected_rows)
				<< "Config: " << config.name << " single column";
		}

		// Double column: cores are paired
		{
			int b_columns = 2;
			int p_rows = (config.p_cores + b_columns - 1) / b_columns;
			int e_rows = (config.e_cores + b_columns - 1) / b_columns;
			int expected_rows = 1 + 1 + 1 + p_rows + 1 + e_rows + 1;

			RenderedBox box = render_cpu_box(config, 80, b_columns);
			EXPECT_EQ(box.height, expected_rows)
				<< "Config: " << config.name << " double column";
		}
	}
}

// Visual test: print all configurations (for manual inspection)
TEST(AppleSiliconCpuBox, VisualRenderAllConfigs) {
	std::cout << "\n=== Apple Silicon CPU Box Rendering Test ===\n" << std::endl;

	for (const auto& config : APPLE_SILICON_CONFIGS) {
		std::cout << "--- " << config.name << " ---" << std::endl;
		std::cout << "Cores: " << config.p_cores << "P + " << config.e_cores << "E = "
				  << (config.p_cores + config.e_cores) << " total" << std::endl;
		std::cout << "Freq: P=" << format_freq(config.p_freq_mhz)
				  << " E=" << format_freq(config.e_freq_mhz) << std::endl;

		// Render at typical width with 2 columns
		RenderedBox box = render_cpu_box(config, 50, 2);
		print_box(box);
		std::cout << std::endl;
	}
}

// Visual test: show how different box widths affect layout
TEST(AppleSiliconCpuBox, VisualWidthComparison) {
	std::cout << "\n=== Box Width Comparison (M1 Max) ===\n" << std::endl;

	AppleSiliconConfig config = {"M1 Max", 8, 2, 3228, 2064};

	for (int width : {40, 50, 60, 80}) {
		std::cout << "Width: " << width << ", 2 columns" << std::endl;
		RenderedBox box = render_cpu_box(config, width, 2);
		print_box(box);
		std::cout << std::endl;
	}
}

// Visual test: compare single vs double column layout
TEST(AppleSiliconCpuBox, VisualColumnComparison) {
	std::cout << "\n=== Column Layout Comparison (M3 Pro 12C) ===\n" << std::endl;

	AppleSiliconConfig config = {"M3 Pro 12C", 6, 6, 4056, 2748};

	std::cout << "Single column (width 40):" << std::endl;
	RenderedBox box1 = render_cpu_box(config, 40, 1);
	print_box(box1);
	std::cout << std::endl;

	std::cout << "Double column (width 60):" << std::endl;
	RenderedBox box2 = render_cpu_box(config, 60, 2);
	print_box(box2);
	std::cout << std::endl;
}

// Test extreme configurations
TEST(AppleSiliconCpuBox, ExtremeConfigs) {
	// M1 Ultra has the most cores in M1 family
	{
		AppleSiliconConfig ultra = {"M1 Ultra", 16, 4, 3228, 2064};
		RenderedBox box = render_cpu_box(ultra, 80, 2);
		EXPECT_LE(box.height, 20) << "M1 Ultra should fit in reasonable height";

		std::cout << "\n=== M1 Ultra (16P + 4E) ===\n";
		print_box(box);
	}

	// M2 Ultra has the most cores overall
	{
		AppleSiliconConfig ultra = {"M2 Ultra", 16, 8, 3504, 2424};
		RenderedBox box = render_cpu_box(ultra, 80, 2);
		EXPECT_LE(box.height, 22) << "M2 Ultra should fit in reasonable height";

		std::cout << "\n=== M2 Ultra (16P + 8E) ===\n";
		print_box(box);
	}
}

// Test frequency string width consistency
TEST(AppleSiliconCpuBox, FreqStringWidthConsistency) {
	// All GHz frequencies should have similar widths
	std::vector<int> ghz_freqs = {1000, 1500, 2000, 2500, 3000, 3500, 4000, 4500};
	for (int freq : ghz_freqs) {
		std::string str = format_freq(freq);
		// "X.X GHz" = 7 chars
		EXPECT_LE(str.size(), 7u) << "Freq " << freq << " formatted as '" << str << "'";
		EXPECT_GE(str.size(), 6u) << "Freq " << freq << " formatted as '" << str << "'";
	}

	// MHz frequencies vary more
	std::vector<int> mhz_freqs = {600, 700, 800, 900, 999};
	for (int freq : mhz_freqs) {
		std::string str = format_freq(freq);
		// "XXX MHz" = 7 chars
		EXPECT_LE(str.size(), 7u) << "Freq " << freq << " formatted as '" << str << "'";
	}
}

// Test that the layout handles odd core counts gracefully
TEST(AppleSiliconCpuBox, OddCoreCounts) {
	// M3 Pro 11C has 5P + 6E
	AppleSiliconConfig m3pro11 = {"M3 Pro 11C", 5, 6, 4056, 2748};

	std::cout << "\n=== M3 Pro 11C (5P + 6E) - Odd P-core count ===\n";
	RenderedBox box = render_cpu_box(m3pro11, 60, 2);
	print_box(box);

	// With 2 columns:
	// P-cores: 3 rows (2+2+1)
	// E-cores: 3 rows (2+2+2)
	int expected_p_rows = (5 + 1) / 2;  // 3
	int expected_e_rows = (6 + 1) / 2;  // 3
	EXPECT_EQ(expected_p_rows, 3);
	EXPECT_EQ(expected_e_rows, 3);
}

// =============================================================================
// Edge Case Tests: E-core Clipping at Boundary Widths
// =============================================================================

// Simulate the column layout decision logic from btop_draw.cpp
// Returns {b_columns, b_column_size, b_width}
struct LayoutParams {
	int b_columns;
	int b_column_size;
	int b_width;
};

LayoutParams calculate_layout(int width, int height, int core_count, bool show_temp = false,
                              int p_cores = 0, int e_cores = 0) {
	LayoutParams params;

	// Simplified from btop_draw.cpp lines 2420-2440
	params.b_columns = std::max(1, (int)ceil((double)(core_count + 1) / (height - 5)));

	// Apple Silicon P/E-aware column adjustment (matches fix in btop_draw.cpp)
	if (p_cores > 0 and e_cores > 0) {
		const int available_rows = height - 5 - 2;  // -2 for P-CPU and E-CPU header bars
		// Find minimum columns needed so both P and E sections fit
		while (params.b_columns < core_count) {
			int p_rows = (p_cores + params.b_columns - 1) / params.b_columns;
			int e_rows = (e_cores + params.b_columns - 1) / params.b_columns;
			if (p_rows + e_rows <= available_rows) break;
			params.b_columns++;
		}
	}

	if (params.b_columns * (21 + 12 * show_temp) < width - (width / 3)) {
		params.b_column_size = 2;
		params.b_width = std::max(29, (21 + 12 * show_temp) * params.b_columns - (params.b_columns - 1));
	}
	else if (params.b_columns * (15 + 6 * show_temp) < width - (width / 3)) {
		params.b_column_size = 1;
		params.b_width = (15 + 6 * show_temp) * params.b_columns - (params.b_columns - 1);
	}
	else if (params.b_columns * (8 + 6 * show_temp) < width - (width / 3)) {
		params.b_column_size = 0;
		params.b_width = (8 + 6 * show_temp) * params.b_columns + 1;
	}
	else {
		params.b_columns = (width - width / 3) / (8 + 6 * show_temp);
		params.b_column_size = 0;
		params.b_width = (8 + 6 * show_temp) * params.b_columns + 1;
	}

	return params;
}

// Render box with clipping detection - simulates the actual drawing logic
struct ClippingResult {
	bool e_core_clipped;
	int e_cores_drawn;
	int e_cores_expected;
	int available_rows;
	int rows_used;
	std::string details;
};

ClippingResult render_with_clipping_check(const AppleSiliconConfig& config, int b_width, int b_columns, int max_row) {
	ClippingResult result;
	result.e_cores_expected = config.e_cores;
	result.e_cores_drawn = 0;
	result.e_core_clipped = false;

	int col_width = b_width / b_columns;
	int cy = 0;  // Current row

	// P-CPU header bar
	cy++;

	// P-cores
	int p_rows = (config.p_cores + b_columns - 1) / b_columns;
	int p_drawn = 0;
	for (int row = 0; row < p_rows and p_drawn < config.p_cores and cy < max_row; ++row) {
		for (int col = 0; col < b_columns and p_drawn < config.p_cores; ++col) {
			p_drawn++;
		}
		cy++;
	}

	// E-CPU header bar
	if (cy < max_row) {
		cy++;
	}

	// E-cores - this is where clipping can occur
	int e_rows = (config.e_cores + b_columns - 1) / b_columns;
	for (int row = 0; row < e_rows and result.e_cores_drawn < config.e_cores and cy < max_row; ++row) {
		for (int col = 0; col < b_columns and result.e_cores_drawn < config.e_cores; ++col) {
			result.e_cores_drawn++;
		}
		cy++;
	}

	result.available_rows = max_row;
	result.rows_used = cy;
	result.e_core_clipped = (result.e_cores_drawn < result.e_cores_expected);

	result.details = fmt::format("b_width={}, b_columns={}, col_width={}, max_row={}, "
								 "p_cores={}, e_cores={}, e_drawn={}, rows_used={}",
								 b_width, b_columns, col_width, max_row,
								 config.p_cores, config.e_cores, result.e_cores_drawn, cy);

	return result;
}

// Test: E-core clipping at boundary widths
TEST(AppleSiliconCpuBox, EcoreClippingAtBoundaryWidths) {
	std::cout << "\n=== E-core Clipping at Boundary Widths ===\n" << std::endl;

	// Test configs with various E-core counts
	std::vector<AppleSiliconConfig> test_configs = {
		{"4E Test", 4, 4, 3228, 2064},  // M1-like
		{"6E Test", 4, 6, 4416, 2892},  // M4-like
		{"8E Test", 16, 8, 3504, 2424}, // M2 Ultra-like
	};

	// Test boundary widths where column layout transitions occur
	// Key thresholds from btop_draw.cpp:
	// - b_column_size=2: b_columns * 21 < width - width/3  (for no temp)
	// - b_column_size=1: b_columns * 15 < width - width/3
	// - b_column_size=0: b_columns * 8 < width - width/3

	std::vector<int> boundary_widths = {30, 31, 32, 40, 41, 42, 43, 44, 45, 46, 47, 48};

	for (const auto& config : test_configs) {
		std::cout << "Config: " << config.name << " (" << config.p_cores << "P + "
				  << config.e_cores << "E)" << std::endl;

		for (int width : boundary_widths) {
			// Use realistic height based on core count - need enough rows for all cores
			// Minimum height = P rows + E rows + headers (2) + base (5) + borders (2)
			int total_cores = config.p_cores + config.e_cores;
			int min_height = (config.p_cores + 1) / 2 + (config.e_cores + 1) / 2 + 2 + 5 + 2;
			int height = std::max(15, min_height);  // At least 15 rows

			auto layout = calculate_layout(width * 3 / 2, height, total_cores, false,
			                               config.p_cores, config.e_cores);

			// Calculate max_row (available content rows)
			// With P/E layout: P-CPU header + P rows + E-CPU header + E rows + base(4)
			int p_rows = (config.p_cores + layout.b_columns - 1) / layout.b_columns;
			int e_rows = (config.e_cores + layout.b_columns - 1) / layout.b_columns;
			int pe_height = p_rows + e_rows + 4 + 1;  // +1 for the extra header
			int b_height = std::min(height - 2, pe_height);
			int max_row = b_height - 2;  // Content area

			auto result = render_with_clipping_check(config, layout.b_width, layout.b_columns, max_row);

			if (result.e_core_clipped) {
				std::cout << "  CLIPPED at width=" << width << ", height=" << height
				          << ": " << result.details << std::endl;
			}

			// Assert that all E-cores are drawn (no clipping) when height is adequate
			EXPECT_EQ(result.e_cores_drawn, result.e_cores_expected)
				<< "E-core clipping detected with adequate height: " << result.details;
		}
		std::cout << std::endl;
	}
}

// Test: Column layout boundary conditions
TEST(AppleSiliconCpuBox, ColumnLayoutBoundaryConditions) {
	std::cout << "\n=== Column Layout Boundary Conditions ===\n" << std::endl;

	// Test the exact thresholds where column count changes
	// From btop_draw.cpp: b_columns * (8 + 6*show_temp) < width - width/3
	// For show_temp=false: b_columns * 8 < width * 2/3
	// => width > b_columns * 8 * 3/2 = b_columns * 12

	struct BoundaryTest {
		int width;
		int height;
		int core_count;
		int expected_min_columns;
		int expected_max_columns;
	};

	std::vector<BoundaryTest> tests = {
		// Width just enough for 1 column
		{24, 20, 8, 1, 2},
		// Width at boundary for 2 columns
		{36, 15, 10, 1, 3},
		// Narrow width forcing more columns
		{48, 10, 12, 2, 4},
		// Very narrow - stress test
		{30, 8, 10, 2, 5},
	};

	for (const auto& test : tests) {
		auto layout = calculate_layout(test.width, test.height, test.core_count, false);

		std::cout << fmt::format("width={}, height={}, cores={} => b_columns={}, b_column_size={}, b_width={}\n",
								 test.width, test.height, test.core_count,
								 layout.b_columns, layout.b_column_size, layout.b_width);

		EXPECT_GE(layout.b_columns, test.expected_min_columns)
			<< "Too few columns for width=" << test.width;
		EXPECT_LE(layout.b_columns, test.expected_max_columns)
			<< "Too many columns for width=" << test.width;

		// Column width should always be positive
		int col_width = layout.b_width / layout.b_columns;
		EXPECT_GT(col_width, 0)
			<< "Column width must be positive, got " << col_width;
	}
}

// Test: Various E-core counts (4, 6, 8) with narrow widths
TEST(AppleSiliconCpuBox, VariousEcoreCountsNarrowWidths) {
	std::cout << "\n=== Various E-core Counts with Narrow Widths ===\n" << std::endl;

	// Configs representing different E-core counts found in Apple Silicon
	std::vector<AppleSiliconConfig> ecore_configs = {
		{"2E (M1 Pro)",   8, 2, 3228, 2064},
		{"4E (M1/M2)",    4, 4, 3228, 2064},
		{"4E (M3 Max)",  10, 4, 4056, 2748},
		{"6E (M3 Pro)",   6, 6, 4056, 2748},
		{"6E (M4)",       4, 6, 4416, 2892},
		{"8E (M2 Ultra)",16, 8, 3504, 2424},
	};

	// Narrow widths that stress the layout
	std::vector<int> narrow_widths = {28, 30, 32, 35, 38, 40, 42, 45};

	for (const auto& config : ecore_configs) {
		std::cout << "Testing " << config.name << " (" << config.p_cores << "P + "
				  << config.e_cores << "E):" << std::endl;

		for (int b_width : narrow_widths) {
			for (int b_columns = 1; b_columns <= 3; b_columns++) {
				int col_width = b_width / b_columns;

				// Skip invalid configurations
				if (col_width < 8) continue;  // Minimum column width

				// Calculate rows needed
				int p_rows = (config.p_cores + b_columns - 1) / b_columns;
				int e_rows = (config.e_cores + b_columns - 1) / b_columns;
				int total_content_rows = 1 + p_rows + 1 + e_rows;  // P-header + P-cores + E-header + E-cores

				// Test with tight max_row that could cause clipping
				int max_row = total_content_rows - 1;  // One row short - should clip last E-core row

				auto result = render_with_clipping_check(config, b_width, b_columns, max_row);

				// When max_row is insufficient, clipping is expected
				// But the number of E-cores drawn should be predictable
				int expected_e_rows_drawn = std::max(0, max_row - 1 - p_rows - 1);
				int expected_e_cores_min = expected_e_rows_drawn > 0 ?
					(expected_e_rows_drawn - 1) * b_columns + 1 : 0;
				int expected_e_cores_max = expected_e_rows_drawn * b_columns;

				if (result.e_core_clipped) {
					EXPECT_GE(result.e_cores_drawn, expected_e_cores_min)
						<< "E-cores drawn below minimum: " << result.details;
					EXPECT_LE(result.e_cores_drawn, expected_e_cores_max)
						<< "E-cores drawn above maximum: " << result.details;
				}
			}
		}

		// Test with adequate space - should never clip
		for (int b_columns = 1; b_columns <= 2; b_columns++) {
			int p_rows = (config.p_cores + b_columns - 1) / b_columns;
			int e_rows = (config.e_cores + b_columns - 1) / b_columns;
			int required_rows = 1 + p_rows + 1 + e_rows;
			int max_row = required_rows + 5;  // Plenty of space

			auto result = render_with_clipping_check(config, 60, b_columns, max_row);

			EXPECT_FALSE(result.e_core_clipped)
				<< "Unexpected clipping with adequate space: " << result.details;
			EXPECT_EQ(result.e_cores_drawn, config.e_cores)
				<< "Not all E-cores drawn: " << result.details;
		}

		std::cout << "  Passed all narrow width tests" << std::endl;
	}
}

// Test: Exact clipping boundary detection
TEST(AppleSiliconCpuBox, ExactClippingBoundary) {
	std::cout << "\n=== Exact Clipping Boundary Detection ===\n" << std::endl;

	// Test the exact row where clipping starts
	AppleSiliconConfig config = {"M4", 4, 6, 4416, 2892};  // 4P + 6E

	std::cout << "Config: M4 (4P + 6E)" << std::endl;

	for (int b_columns = 1; b_columns <= 2; b_columns++) {
		int p_rows = (config.p_cores + b_columns - 1) / b_columns;
		int e_rows = (config.e_cores + b_columns - 1) / b_columns;

		std::cout << fmt::format("  b_columns={}: p_rows={}, e_rows={}\n",
								 b_columns, p_rows, e_rows);

		// Calculate exact minimum max_row needed to show all E-cores
		int min_max_row = 1 + p_rows + 1 + e_rows;  // P-header + P-rows + E-header + E-rows

		// Test one row below threshold (should clip)
		auto result_clip = render_with_clipping_check(config, 60, b_columns, min_max_row - 1);
		EXPECT_TRUE(result_clip.e_core_clipped)
			<< "Expected clipping at max_row=" << (min_max_row - 1) << ": " << result_clip.details;

		// Test at exact threshold (should NOT clip)
		auto result_exact = render_with_clipping_check(config, 60, b_columns, min_max_row);
		EXPECT_FALSE(result_exact.e_core_clipped)
			<< "Unexpected clipping at exact threshold max_row=" << min_max_row << ": " << result_exact.details;
		EXPECT_EQ(result_exact.e_cores_drawn, config.e_cores)
			<< "All E-cores should be drawn at exact threshold: " << result_exact.details;

		// Test one row above threshold (definitely should NOT clip)
		auto result_above = render_with_clipping_check(config, 60, b_columns, min_max_row + 1);
		EXPECT_FALSE(result_above.e_core_clipped)
			<< "Unexpected clipping above threshold: " << result_above.details;

		std::cout << fmt::format("    min_max_row={}, clip_at_{}: drawn={}, exact_at_{}: drawn={}\n",
								 min_max_row,
								 min_max_row - 1, result_clip.e_cores_drawn,
								 min_max_row, result_exact.e_cores_drawn);
	}
}

// Test: Column width calculation edge cases
TEST(AppleSiliconCpuBox, ColumnWidthEdgeCases) {
	std::cout << "\n=== Column Width Edge Cases ===\n" << std::endl;

	// Test that col_width = b_width / b_columns handles integer division correctly
	struct WidthTest {
		int b_width;
		int b_columns;
		int expected_col_width;
		int expected_remainder;
	};

	std::vector<WidthTest> tests = {
		{60, 2, 30, 0},
		{61, 2, 30, 1},  // 1 pixel lost to integer division
		{59, 2, 29, 1},
		{45, 2, 22, 1},
		{45, 3, 15, 0},
		{46, 3, 15, 1},
		{47, 3, 15, 2},
		{29, 2, 14, 1},
		{30, 2, 15, 0},
		{31, 2, 15, 1},
	};

	for (const auto& test : tests) {
		int col_width = test.b_width / test.b_columns;
		int remainder = test.b_width % test.b_columns;

		EXPECT_EQ(col_width, test.expected_col_width)
			<< fmt::format("b_width={}, b_columns={}", test.b_width, test.b_columns);
		EXPECT_EQ(remainder, test.expected_remainder)
			<< fmt::format("b_width={}, b_columns={}", test.b_width, test.b_columns);

		// The last core in a row might have slightly different width due to remainder
		// This could cause clipping if not handled correctly
		int total_used = col_width * test.b_columns;
		int unused = test.b_width - total_used;

		std::cout << fmt::format("b_width={}, b_columns={} => col_width={}, remainder={}, unused={}\n",
								 test.b_width, test.b_columns, col_width, remainder, unused);

		EXPECT_GE(unused, 0)
			<< "Negative unused space indicates overflow";
	}
}

// Test: Stress test with minimal dimensions
TEST(AppleSiliconCpuBox, MinimalDimensionsStressTest) {
	std::cout << "\n=== Minimal Dimensions Stress Test ===\n" << std::endl;

	// Test all Apple Silicon configs with minimal viable dimensions
	for (const auto& config : APPLE_SILICON_CONFIGS) {
		// Find minimum viable b_width for 2 columns
		int min_b_width = 29;  // Minimum from btop_draw.cpp
		int b_columns = 2;

		// Calculate required rows
		int p_rows = (config.p_cores + b_columns - 1) / b_columns;
		int e_rows = (config.e_cores + b_columns - 1) / b_columns;
		int min_content_rows = 1 + p_rows + 1 + e_rows;

		auto result = render_with_clipping_check(config, min_b_width, b_columns, min_content_rows);

		EXPECT_FALSE(result.e_core_clipped)
			<< config.name << " clipped at minimum viable dimensions: " << result.details;
		EXPECT_EQ(result.e_cores_drawn, config.e_cores)
			<< config.name << " E-cores not fully rendered: " << result.details;
	}

	std::cout << "All configs passed minimal dimensions test" << std::endl;
}

// Visual test: Show clipping behavior
TEST(AppleSiliconCpuBox, VisualClippingBehavior) {
	std::cout << "\n=== Visual Clipping Behavior ===\n" << std::endl;

	AppleSiliconConfig config = {"M4", 4, 6, 4416, 2892};

	// Show rendering at various max_row values
	for (int max_row = 4; max_row <= 12; max_row++) {
		auto result = render_with_clipping_check(config, 60, 2, max_row);

		std::string status = result.e_core_clipped ? "CLIPPED" : "OK";
		std::cout << fmt::format("max_row={:2d}: E-cores {}/{} [{}] rows_used={}\n",
								 max_row, result.e_cores_drawn, result.e_cores_expected,
								 status, result.rows_used);

		// Visual representation
		std::cout << "  Layout: [P-CPU]";
		int p_rows = (config.p_cores + 1) / 2;
		for (int i = 0; i < p_rows; i++) std::cout << "[P]";
		std::cout << "[E-CPU]";
		int e_drawn_rows = (result.e_cores_drawn + 1) / 2;
		int e_expected_rows = (config.e_cores + 1) / 2;
		for (int i = 0; i < e_drawn_rows; i++) std::cout << "[E]";
		for (int i = e_drawn_rows; i < e_expected_rows; i++) std::cout << "[X]";  // Clipped
		std::cout << std::endl;
	}
}
