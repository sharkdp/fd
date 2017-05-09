#include <iostream>
#include <regex>
#include <boost/filesystem.hpp>

namespace fs = boost::filesystem;

static const std::string ANSI_PURPLE = "\x1b[35;06m";
static const std::string ANSI_CYAN = "\x1b[36;01m";
static const std::string ANSI_RESET = "\x1b[0m";

void printPath(const fs::path& path) {
    if (fs::is_symlink(path)) {
        std::cout << ANSI_PURPLE;
    } else if (fs::is_directory(path)) {
        std::cout << ANSI_CYAN;
    }

    std::cout << path.string();

    std::cout << ANSI_RESET << std::endl;

}

void findFiles(const std::regex& pattern) {
    const fs::path& currentPath = fs::current_path();

    for (auto& entry: fs::recursive_directory_iterator(currentPath)) {
        const fs::path& path = entry.path().lexically_relative(currentPath);

        if (std::regex_search(path.string(), pattern)) {
            printPath(path);
        }
    }
}

int main(int argc, char* argv[]) {
    std::string argument;

    if (argc == 1) {
        argument = "";
    } else if (argc == 2) {
        argument = argv[1];
    }

    if (argc > 2 || argument == "-h" || argument == "--help") {
        std::cerr << "Usage: fnd [PATTERN]" << std::endl;
        return 1;
    }

    // try to parse the argument as a regex
    try {
        std::regex re(argument);

        findFiles(re);
    }
    catch (const std::regex_error& e) {
        std::cerr << "Regex error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
