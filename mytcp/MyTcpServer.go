package mytcp

import (
	"fmt"
	"io"
	"syscall"

	//"math/rand"
	"net"
	//"syscall"
	"time"
)

func MyTcpServer(str string) {
	IPAddrString := str
	tcpAddr, err := net.ResolveTCPAddr("tcp4", IPAddrString)

	CheckError(err)
	listener, err := net.ListenTCP("tcp4", tcpAddr)
	CheckError(err)
	//tools.CheckError(err)
	//var file *os.File
	//var logFileName string = "/root/Servertcp.log"
	//if runtime.GOOS == "linux" {
	//	logFileName = "/root/Servertcp.log"
	//
	//} else if runtime.GOOS == "windows"{
	//	logFileName = "C:\\Servertcp.log"
	//}
	for {
		conn, err := listener.Accept()
		if err != nil {
			continue
		}
		go HandleConn(conn)
	}
}

func HandleConn(conn net.Conn) {
	if i, ok := conn.(*net.TCPConn); ok {
		err := i.SetKeepAlive(false)
		CheckError(err)
	}
	defer conn.Close()
	daytime := time.Now().String()
	daytime = fmt.Sprintf("Server Time: %s", daytime)
	conn.Write([]byte(daytime))
	//tag :=	rand.Intn(100)

	fmt.Println("[New] Tcp Connection:", conn.RemoteAddr())
	//remoteAddr := conn.RemoteAddr()
	go circlePrint("Server: ", 1000)
	for {
		hasData, e := connCheck(conn)
		//fmt.Println("return error", e)
		if e == io.EOF {
			fmt.Println("[FIN]", conn.RemoteAddr(), "Closed")
			return
		} else if e == syscall.ECONNRESET {
			fmt.Printf("[RET] %s: %s\n", e, conn.RemoteAddr())
			return
		}
		if hasData {
			//_ = conn.SetReadDeadline(time.Now().Add(time.Millisecond * 1005))
			content := make([]byte, 1024)

			_, TCPErr := conn.Read(content)
			if TCPErr != nil { // i/o timeout
				//fmt.Println("--",TCPErr)
				if TCPErr == io.EOF {
					fmt.Println("FIN", conn.RemoteAddr(), "Closed")
					return
				}
			} else {
				fmt.Println(string(content))
			}
			//conn.SetReadDeadline(time.Millisecond * 1)
			//conn.SetReadDeadline(time.Time{}) // 取消设置
		}
		//file, err := os.OpenFile(logFileName, os.O_APPEND | os.O_WRONLY | os.O_CREATE, os.ModeAppend)
		//tools.CheckError(err)
		//file.Write([]byte(interval))
		//file.Close()
		//fmt.Printf(" %di", tag)
		//err := conn.SetReadDeadline(time.Now())
		//tools.CheckError(err)
		//_, TCPErr := conn.Read(make([]byte, 1))
		//if TCPErr == io.EOF {
		//	fmt.Println("TCP routine free")
		//	return
		//} else {
		//	fmt.Println(TCPErr)
		//	if neterr, ok := err.(net.Error); ok && (neterr.Timeout() || neterr.Temporary() ){
		//		fmt.Println("test-neterr")
		//	}
		//	//var zero time.Time
		//	//err = conn.SetReadDeadline(zero)
		//	tools.CheckError(err)
		//	conn.SetReadDeadline(time.Now().Add(10 * time.Millisecond))
		//}
		time.Sleep(time.Millisecond * 1000)
	}
}

func connCheck(conn net.Conn) (bool, error) {
	var sysErr error = nil
	var hasData bool = false
	rawConn, err := conn.(syscall.Conn).SyscallConn()
	err = rawConn.Read(func(fd uintptr) bool {
		var buf []byte = []byte{0}

		//fmt.Println(len(buf), cap(buf))
		// sub
		//n, err := syscall.Read(int(fd), buf[:])
		n, _, err := syscall.Recvfrom(int(fd), buf, syscall.MSG_PEEK|syscall.MSG_DONTWAIT)
		//fmt.Println("line:", err, n, buf)
		switch {
		case n == 0 && err == nil:
			sysErr = io.EOF
		case err == syscall.EAGAIN || err == syscall.EWOULDBLOCK:
			sysErr = nil
		//ECONNRESET (54) syscall.Errno
		//case err == syscall.ECONNRESET:connection reset by peer 收到reset报文
		//	sysErr = syscall.ECONNRESET
		case n == 1 && err == nil:
			hasData = true
		default:
			sysErr = err
		}
		return true
	})
	//fmt.Println("syscall raw error", err)
	if err != nil {
		return hasData, err
	}
	return hasData, sysErr
}

func CheckError(err error) {
	if err != nil {
		panic(err)
	}
}
