/* Minimal stand-in for the CMake-generated export header. */
#ifndef FCITXCORE_EXPORT_H
#define FCITXCORE_EXPORT_H

#define FCITXCORE_EXPORT __attribute__((visibility("default")))
#define FCITXCORE_NO_EXPORT __attribute__((visibility("hidden")))
#define FCITXCORE_DEPRECATED __attribute__((__deprecated__))
#define FCITXCORE_DEPRECATED_EXPORT FCITXCORE_EXPORT FCITXCORE_DEPRECATED
#define FCITXCORE_DEPRECATED_NO_EXPORT FCITXCORE_NO_EXPORT FCITXCORE_DEPRECATED

#endif
