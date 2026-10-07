// Fcitx5 TextRelay addon
//
// Runs inside the fcitx5 daemon and commits text to the currently focused
// input context, exactly as if the input method itself had produced it.
// This is the only clean way to insert text into the focused window without
// clipboard round-trips or key-event simulation.
//
// Interface (unix socket, data plane only):
//   Path: $XDG_RUNTIME_DIR/textrelay.sock
//   Protocol: connect, send UTF-8 text, then either shutdown() the write
//   side or terminate the text with '\n'. The server replies one byte:
//   0x01 = committed to the focused input context, 0x00 = nothing focused.
//
// DBus is deliberately NOT used here: it is a control-plane bus, not a data
// plane. All text goes through the local unix socket.

#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>

#include <cerrno>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_map>

#include <unistd.h>

#include <fcitx-utils/event.h>
#include <fcitx-utils/eventloopinterface.h>
#include <fcitx-utils/log.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/instance.h>

namespace fcitx {

namespace {
// Refuse absurdly large payloads over the socket.
constexpr size_t kMaxTextSize = 1 << 20;
} // namespace

class TextRelayAddon : public AddonInstance {
public:
    TextRelayAddon(AddonManager *manager)
        : instance_(manager->instance()) {
        setupSocket();
    }

    ~TextRelayAddon() override {
        for (auto &[fd, client] : clients_) {
            client.event.reset();
            ::close(fd);
        }
        listenEvent_.reset();
        if (listenFd_ >= 0) {
            ::close(listenFd_);
        }
        if (!socketPath_.empty()) {
            ::unlink(socketPath_.c_str());
        }
    }

    bool commit(const std::string &text) {
        auto *ic = instance_->inputContextManager().lastFocusedInputContext();
        if (!ic) {
            FCITX_WARN() << "[textrelay] no focused input context, text dropped";
            return false;
        }
        ic->commitString(text);
        return true;
    }

private:
    void setupSocket() {
        const char *runtime = ::getenv("XDG_RUNTIME_DIR");
        const std::string dir = (runtime && *runtime) ? runtime : "/tmp";
        socketPath_ = dir + "/textrelay.sock";

        listenFd_ = ::socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
        if (listenFd_ < 0) {
            FCITX_WARN() << "[textrelay] socket() failed: " << ::strerror(errno);
            return;
        }
        // A leftover socket from a previous instance is ours to replace.
        ::unlink(socketPath_.c_str());
        struct sockaddr_un addr {};
        addr.sun_family = AF_UNIX;
        ::strncpy(addr.sun_path, socketPath_.c_str(), sizeof(addr.sun_path) - 1);
        if (::bind(listenFd_, reinterpret_cast<struct sockaddr *>(&addr),
                   sizeof(addr)) < 0 ||
            ::listen(listenFd_, 8) < 0) {
            FCITX_WARN() << "[textrelay] bind/listen " << socketPath_
                         << " failed: " << ::strerror(errno);
            ::close(listenFd_);
            listenFd_ = -1;
            return;
        }
        ::chmod(socketPath_.c_str(), 0600);

        listenEvent_ = instance_->eventLoop().addIOEvent(
            listenFd_, {IOEventFlag::In},
            [this](EventSourceIO *, int fd, IOEventFlags) {
                while (true) {
                    int cfd = ::accept4(fd, nullptr, nullptr,
                                        SOCK_NONBLOCK | SOCK_CLOEXEC);
                    if (cfd < 0) {
                        break; // EAGAIN: all pending connections accepted
                    }
                    addClient(cfd);
                }
                return true;
            });
        FCITX_INFO() << "[textrelay] listening on " << socketPath_;
    }

    struct Client {
        std::string buf;
        std::unique_ptr<EventSourceIO> event;
    };

    void addClient(int cfd) {
        auto [iter, inserted] =
            clients_.emplace(cfd, Client{});
        if (!inserted) {
            ::close(cfd);
            return;
        }
        iter->second.event = instance_->eventLoop().addIOEvent(
            cfd, {IOEventFlag::In, IOEventFlag::Err, IOEventFlag::Hup},
            [this, cfd](EventSourceIO *, int, IOEventFlags flags) {
                handleClientIO(cfd, flags);
                return true;
            });
    }

    void handleClientIO(int cfd, IOEventFlags flags) {
        auto iter = clients_.find(cfd);
        if (iter == clients_.end()) {
            return;
        }
        if (flags.test(IOEventFlag::Err) || flags.test(IOEventFlag::Hup)) {
            finishClient(cfd, /*reply=*/false);
            return;
        }
        char chunk[4096];
        while (true) {
            ssize_t n = ::read(cfd, chunk, sizeof(chunk));
            if (n > 0) {
                auto &buf = iter->second.buf;
                buf.append(chunk, n);
                if (buf.size() > kMaxTextSize) {
                    finishClient(cfd, /*reply=*/false);
                    return;
                }
                auto pos = buf.find('\n');
                if (pos != std::string::npos) {
                    finishWith(cfd, buf.substr(0, pos));
                    return;
                }
            } else if (n == 0) {
                // Peer shut down its write side: message is complete.
                finishWith(cfd, iter->second.buf);
                return;
            } else if (errno == EAGAIN || errno == EWOULDBLOCK) {
                return; // wait for more data
            } else if (errno != EINTR) {
                finishClient(cfd, /*reply=*/false);
                return;
            }
        }
    }

    // Complete text received (via '\n' or EOF): commit and answer.
    void finishWith(int cfd, const std::string &text) {
        bool ok = true;
        if (!text.empty()) {
            ok = commit(text);
        }
        finishClient(cfd, /*reply=*/true, ok);
    }

    void finishClient(int cfd, bool reply, bool ok = false) {
        if (reply) {
            const uint8_t answer = ok ? 0x01 : 0x00;
            ssize_t n = 0;
            do {
                n = ::write(cfd, &answer, 1);
            } while (n < 0 && errno == EINTR);
        }
        auto iter = clients_.find(cfd);
        if (iter != clients_.end()) {
            clients_.erase(iter); // removes the IO event source
        }
        ::close(cfd);
    }

    Instance *instance_;

    std::string socketPath_;
    int listenFd_ = -1;
    std::unique_ptr<EventSourceIO> listenEvent_;
    std::unordered_map<int, Client> clients_;
};

class TextRelayAddonFactory : public AddonFactory {
    AddonInstance *create(AddonManager *manager) override {
        return new TextRelayAddon(manager);
    }
};

} // namespace fcitx

FCITX_ADDON_FACTORY(fcitx::TextRelayAddonFactory)