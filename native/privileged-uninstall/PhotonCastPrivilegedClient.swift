import Foundation
import Darwin
import Security
import ServiceManagement

func emit(_ response: PrivilegedResponse) -> Never {
    FileHandle.standardOutput.write(encodeResponse(response))
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
    exit(response.ok ? 0 : 1)
}

func newRequestID() -> String { UUID().uuidString }

func usage(_ requestID: String) -> Never {
    emit(PrivilegedResponse(
        ok: false,
        requestID: requestID,
        code: "usage",
        message: "usage: photoncast-privileged-client status|bless|uninstall <path> [trashFirst|deleteConfirmed]",
        operation: nil
    ))
}

func blessHelper(requestID: String) -> Never {
    var authRef: AuthorizationRef?
    let authStatus = AuthorizationCreate(nil, nil, [], &authRef)
    guard authStatus == errAuthorizationSuccess, let authRef else {
        emit(PrivilegedResponse(ok: false, requestID: requestID, code: "authorization-failed", message: "AuthorizationCreate failed with status \(authStatus)", operation: nil))
    }

    var error: Unmanaged<CFError>?
    let blessed = callSMJobBless(helperLabel: helperBundleID, authRef: authRef, error: &error)
    if blessed {
        emit(PrivilegedResponse(ok: true, requestID: requestID, code: "blessed", message: "helper installed", operation: nil))
    }

    let message = error?.takeRetainedValue().localizedDescription ?? "SMJobBless failed"
    emit(PrivilegedResponse(ok: false, requestID: requestID, code: "bless-failed", message: message, operation: nil))
}

private typealias SMJobBlessFunction = @convention(c) (
    CFString,
    CFString,
    AuthorizationRef,
    UnsafeMutablePointer<Unmanaged<CFError>?>?
) -> Bool

func callSMJobBless(
    helperLabel: String,
    authRef: AuthorizationRef,
    error: UnsafeMutablePointer<Unmanaged<CFError>?>?
) -> Bool {
    guard let framework = dlopen(
        "/System/Library/Frameworks/ServiceManagement.framework/ServiceManagement",
        RTLD_NOW
    ) else {
        return false
    }
    defer { dlclose(framework) }

    guard let symbol = dlsym(framework, "SMJobBless") else {
        return false
    }

    let function = unsafeBitCast(symbol, to: SMJobBlessFunction.self)
    return function(kSMDomainSystemLaunchd, helperLabel as CFString, authRef, error)
}

func sendToHelper(_ request: PrivilegedRequest) -> Never {
    let connection = NSXPCConnection(machServiceName: helperMachServiceName, options: .privileged)
    connection.remoteObjectInterface = NSXPCInterface(with: PhotonCastPrivilegedUninstallProtocol.self)
    connection.resume()

    let semaphore = DispatchSemaphore(value: 0)
    var finalResponse: PrivilegedResponse?

    let proxy = connection.remoteObjectProxyWithErrorHandler { error in
        finalResponse = PrivilegedResponse(ok: false, requestID: request.requestID, code: "xpc-error", message: error.localizedDescription, operation: nil)
        semaphore.signal()
    } as? PhotonCastPrivilegedUninstallProtocol

    guard let proxy else {
        connection.invalidate()
        emit(PrivilegedResponse(ok: false, requestID: request.requestID, code: "xpc-error", message: "failed to create helper proxy", operation: nil))
    }

    do {
        let data = try JSONEncoder().encode(request)
        proxy.handle(data) { responseData in
            finalResponse = try? JSONDecoder().decode(PrivilegedResponse.self, from: responseData)
            semaphore.signal()
        }
    } catch {
        connection.invalidate()
        emit(PrivilegedResponse(ok: false, requestID: request.requestID, code: "encode-failed", message: error.localizedDescription, operation: nil))
    }

    _ = semaphore.wait(timeout: .now() + 30)
    connection.invalidate()
    emit(finalResponse ?? PrivilegedResponse(ok: false, requestID: request.requestID, code: "timeout", message: "helper did not respond", operation: nil))
}

@main
struct PhotonCastPrivilegedClientMain {
    static func main() {
        let requestID = newRequestID()
        let arguments = Array(CommandLine.arguments.dropFirst())
        guard let commandName = arguments.first else { usage(requestID) }

        if commandName == "bless" {
            blessHelper(requestID: requestID)
        }

        guard let command = PrivilegedCommand(rawValue: commandName) else { usage(requestID) }
        let path = arguments.dropFirst().first
        let mode = arguments.dropFirst().dropFirst().first.flatMap(UninstallMode.init(rawValue:))
        sendToHelper(PrivilegedRequest(command: command, path: path, mode: mode, requestID: requestID))
    }
}
