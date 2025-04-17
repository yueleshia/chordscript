package main

import (
    "fmt"
    "log"
    "net/http"
)

//run: go run ./%
func main() {
    // API routes
    http.Handle("/", http.FileServer(http.Dir("./")))

    port := ":5000"
    fmt.Println("Server is running on port" + port)
    // Start server on port specified above
    log.Fatal(http.ListenAndServe(port, nil))
}
