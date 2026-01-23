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

#include <gtest/gtest.h>
#include "../src/osx/ioreport.hpp"

// Test IOReport initialization and cleanup
TEST(IOReportTest, InitAndCleanup) {
	// IOReport::init() might fail if not on Apple Silicon or proper privileges/environment
	// so we check boolean result but don't fail test if false (unless we know we are on M1)
	bool available = IOReport::init();
	
	if (available) {
		EXPECT_TRUE(IOReport::is_available());
		auto freqs = IOReport::get_cpu_frequencies();
		// Frequency might be 0 if just initialized and no delta yet
		// or >0 if it waited or logic allows
		
		IOReport::cleanup();
		EXPECT_FALSE(IOReport::is_available());
	} else {
		EXPECT_FALSE(IOReport::is_available());
	}
}

// Test CPU Frequency fetch when not available
TEST(IOReportTest, GetCpuFrequenciesWhenNotAvailable) {
	// Ensure cleanup
	IOReport::cleanup();
	
	auto freqs = IOReport::get_cpu_frequencies();
	EXPECT_EQ(freqs.first, 0u);
	EXPECT_EQ(freqs.second, 0u);
}
