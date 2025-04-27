use prelude::*;

errors!(
    SocketError,
    ConnectError,
    FcntlError,
    SetSockOpt,
    BindError,
    ListenError,
    GetSockNameError,
    AcceptError,
    EAgain
);
