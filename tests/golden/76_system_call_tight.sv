module m;
  logic [31:0] n;
  initial begin
    n = $clog2 ( 16 );
    $display ("hello %d", n);
  end
endmodule
// expected -----
module m;
  logic [31:0] n;
  initial begin
    n = $clog2(16);
    $display("hello %d", n);
  end
endmodule
