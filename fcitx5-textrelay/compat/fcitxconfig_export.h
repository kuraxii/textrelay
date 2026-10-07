/* Minimal stand-in for the CMake-generated export header. */
#ifndef FCITXCONFIG_EXPORT_H
#define FCITXCONFIG_EXPORT_H

#define FCITXCONFIG_EXPORT __attribute__((visibility("default")))
#define FCITXCONFIG_NO_EXPORT __attribute__((visibility("hidden")))
#define FCITXCONFIG_DEPRECATED __attribute__((__deprecated__))
#define FCITXCONFIG_DEPRECATED_EXPORT FCITXCONFIG_EXPORT FCITXCONFIG_DEPRECATED
#define FCITXCONFIG_DEPRECATED_NO_EXPORT FCITXCONFIG_NO_EXPORT FCITXCONFIG_DEPRECATED

#endif
