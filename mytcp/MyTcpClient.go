package mytcp

import (
	"fmt"
	"io"
	"net"
	"syscall"

	//"syscall"

	"time"
)

func MyTcpClient(str string) {

	IPAddrString := str
	tcpAddr, err := net.ResolveTCPAddr("tcp4", IPAddrString)
	CheckError(err)
	conn, err := net.DialTCP("tcp4", nil, tcpAddr)
	defer conn.Close()
	conn.SetKeepAlive(false)
	CheckError(err)
	_, err = conn.Write([]byte("ShakeShake"))
	CheckError(err)
	result := make([]byte, 256)
	_, err = conn.Read(result)
	fmt.Println(string(result))
	//tag :=	rand.Intn(100)
	//var file  *os.File;
	//var logFileName string =  "/root/Clienttcp.log"
	//if runtime.GOOS == "linux" {
	//	logFileName = "/root/Clienttcp.log"
	//
	//} else if runtime.GOOS == "windows"{
	//	logFileName = "C:\\Clienttcp.log"
	//}

	startTime := time.Now()
	fmt.Println("Client Time:", startTime.String())
	go circlePrint("Client: ", 1000)
	go func() {
		var str string
		for {
			fmt.Scanln(&str)
			fmt.Println("send:", str)
			conn.Write([]byte(str))
		}
	}()
	for {
		_, e := connCheck(conn)
		if e == io.EOF || e == syscall.ECONNRESET {
			endTime := time.Now()
			var connErrString = ""
			if e == io.EOF {
				connErrString = "EOF"
			} else {
				connErrString = "RST"
			}
			fmt.Printf("----------->END by [%s]\n", connErrString)
			fmt.Println(endTime.String())
			difference := endTime.Sub(startTime)
			fmt.Printf("difference: %v\n", difference)
			return
		}
		//_ = conn.SetReadDeadline(time.Now().Add(time.Millisecond * 1))
		//content := make([]byte, 0);
		//
		//_, TCPErr := conn.Read(content);
		//if TCPErr != nil { // i/o timeout
		//	fmt.Println("--",TCPErr)
		//	if TCPErr == io.EOF {
		//		fmt.Println(conn.RemoteAddr(), "Closed")
		//		return
		//	}
		//} else {
		//	fmt.Println(string(content))
		//}
		time.Sleep(time.Millisecond * 10)

		//interval := fmt.Sprintf("%d-", tag)
		//conn.SetReadDeadline(time.Now())
		//_, TCPErr := conn.Read(make([]byte, 1))
		//if TCPErr == io.EOF {
		//	fmt.Println("TCP Disconnect")

		//	conn.Close()
		//	os.Exit(0)
		//
		//} else {
		//	//fmt.Println("err: ", TCPErr)
		//	if neterr, ok := err.(net.Error); ok && neterr.Timeout() {
		//		fmt.Println("test-neterr")
		//	}

		//	//var zero time.Time
		//	//conn.SetReadDeadline(zero)
		//	conn.SetReadDeadline(time.Now().Add(10 * time.Millisecond))
		//}

		//fmt.Println(interval)
		//file, err := os.OpenFile(logFileName, os.O_APPEND | os.O_WRONLY | os.O_CREATE, os.ModeAppend)
		////_, err := io.WriteString(file, interval)
		//file.Write([]byte(interval))
		//file.Close()

	}

}

func circlePrint(name string, sleepTime int64) {
	var printIndex = 0
	var circleString = [4]string{"-", "\\", "|", "/"}

	for {
		fmt.Printf("\r %s%c[%d;%dm%s%c[0;0m ", name, 0x1B, 4, 33, circleString[printIndex%4], 0x1B)
		printIndex++
		time.Sleep(time.Millisecond * time.Duration(sleepTime))
	}
}
