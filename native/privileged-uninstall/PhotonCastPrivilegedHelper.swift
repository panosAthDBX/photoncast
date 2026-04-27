import Foundation
import os.log

final class HelperDelegate: NSObject, NSXPCListenerDelegate {
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        connection.exportedInterface = NSXPCInterface(with: PhotonCastPrivilegedUninstallProtocol.self)
        connection.exportedObject = PrivilegedUninstallService()
        connection.resume()
        return true
    }
}

final class PrivilegedUninstallService: NSObject, PhotonCastPrivilegedUninstallProtocol {
    private let logger = Logger(subsystem: helperBundleID, category: "uninstall")

    func handle(_ data: Data, withReply reply: @escaping (Data) -> Void) {
        let response: PrivilegedResponse
        do {
            let request = try JSONDecoder().decode(PrivilegedRequest.self, from: data)
            response = try handleRequest(request)
        } catch {
            response = PrivilegedResponse(
                ok: false,
                requestID: "unknown",
                code: "invalid-request",
                message: String(describing: error),
                operation: nil
            )
        }
        reply(encodeResponse(response))
    }

    private func handleRequest(_ request: PrivilegedRequest) throws -> PrivilegedResponse {
        switch request.command {
        case .status:
            return PrivilegedResponse(ok: true, requestID: request.requestID, code: "ok", message: "helper available", operation: nil)
        case .uninstall:
            do {
                let target = try validateApplicationBundle(request.path)
                return performUninstall(target: target, mode: request.mode ?? .trashFirst, requestID: request.requestID)
            } catch {
                logger.error("Rejected privileged uninstall request \(request.requestID, privacy: .public): \(String(describing: error), privacy: .public)")
                return PrivilegedResponse(ok: false, requestID: request.requestID, code: "rejected", message: String(describing: error), operation: nil)
            }
        }
    }

    private func performUninstall(target: URL, mode: UninstallMode, requestID: String) -> PrivilegedResponse {
        logger.info("Privileged uninstall request \(requestID, privacy: .public) for \(target.path, privacy: .public) mode \(mode.rawValue, privacy: .public)")
        switch mode {
        case .trashFirst:
            do {
                var resultingURL: NSURL?
                try FileManager.default.trashItem(at: target, resultingItemURL: &resultingURL)
                return PrivilegedResponse(ok: true, requestID: requestID, code: "trashed", message: "moved to Trash", operation: "trash")
            } catch {
                return PrivilegedResponse(
                    ok: false,
                    requestID: requestID,
                    code: "needs-delete-confirmation",
                    message: "Privileged Trash move failed: \(error.localizedDescription)",
                    operation: nil
                )
            }
        case .deleteConfirmed:
            do {
                try FileManager.default.removeItem(at: target)
                return PrivilegedResponse(ok: true, requestID: requestID, code: "deleted", message: "deleted app bundle", operation: "delete")
            } catch {
                return PrivilegedResponse(ok: false, requestID: requestID, code: "delete-failed", message: error.localizedDescription, operation: nil)
            }
        }
    }
}

@main
struct PhotonCastPrivilegedHelperMain {
    static func main() {
        let delegate = HelperDelegate()
        let listener = NSXPCListener(machServiceName: helperMachServiceName)
        listener.delegate = delegate
        listener.resume()
        RunLoop.main.run()
    }
}
