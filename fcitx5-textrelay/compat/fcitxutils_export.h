/* Minimal stand-in for the CMake-generated export header. */
#ifndef FCITXUTILS_EXPORT_H
#define FCITXUTILS_EXPORT_H

#define FCITXUTILS_EXPORT __attribute__((visibility("default")))
#define FCITXUTILS_NO_EXPORT __attribute__((visibility("hidden")))
#define FCITXUTILS_DEPRECATED __attribute__((__deprecated__))
#define FCITXUTILS_DEPRECATED_EXPORT FCITXUTILS_EXPORT FCITXUTILS_DEPRECATED
#define FCITXUTILS_DEPRECATED_NO_EXPORT FCITXUTILS_NO_EXPORT FCITXUTILS_DEPRECATED

#endif
