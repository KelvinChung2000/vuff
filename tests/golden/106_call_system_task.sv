module m;
  initial begin
    $display ( "hello %d" , x ) ;
    $write("no newline");
    $finish ;
  end
endmodule
// expected -----
module m;
  initial begin
    $display("hello %d", x);
    $write("no newline");
    $finish;
  end
endmodule
