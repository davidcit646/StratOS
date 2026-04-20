cmake_minimum_required(VERSION 3.20)
project(stratbrowser)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

option(STRATBROWSER_ENABLE_GTK "Build GTK4/libadwaita UI when dependencies are available" ON)
option(STRATBROWSER_REQUIRE_GTK "Fail configure when GTK4/libadwaita are missing" ON)

find_package(PkgConfig REQUIRED)
if(STRATBROWSER_ENABLE_GTK)
    pkg_check_modules(GTK4 QUIET IMPORTED_TARGET gtk4)
    pkg_check_modules(ADWAITA QUIET IMPORTED_TARGET libadwaita-1)
endif()

if(GTK4_FOUND AND ADWAITA_FOUND)
    message(STATUS "Building GTK4 UI variant of stratbrowser")
    add_executable(stratbrowser
        src/main_gtk.cpp
        src/BrowserWindow.cpp
    )
    target_link_libraries(stratbrowser
        PRIVATE
        PkgConfig::GTK4
        PkgConfig::ADWAITA
    )
elseif(NOT STRATBROWSER_REQUIRE_GTK)
    message(WARNING "GTK4/libadwaita not found; building fallback stratbrowser binary")
    add_executable(stratbrowser
        src/main_stub.cpp
    )
else()
    message(FATAL_ERROR
        "stratbrowser requires GTK4/libadwaita but they were not found. "
        "Install development packages (e.g. libgtk-4-dev + libadwaita-1-dev) "
        "or configure with -DSTRATBROWSER_REQUIRE_GTK=OFF for fallback mode.")
endif()

install(TARGETS stratbrowser RUNTIME DESTINATION bin)
