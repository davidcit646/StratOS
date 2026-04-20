#include <iostream>
#include <string>

int main(int argc, char** argv) {
    std::cout << "stratbrowser: fallback build (GTK4/libadwaita unavailable on build host)\n";
    if (argc > 1) {
        std::cout << "Requested URL: " << argv[1] << "\n";
    }
    std::cout << "This placeholder keeps StratOS builds runnable while browser UI deps are absent.\n";
    return 0;
}
