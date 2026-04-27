module m;
  initial begin
    $display("a=%0d b=%0d c=%0d",
             aaa, bbb, ccc);
    $display("inline", x, y, z);
  end
endmodule
// expected -----
module m;
  initial begin
    $display(
      "a=%0d b=%0d c=%0d",
      aaa, bbb, ccc
    );
    $display("inline", x, y, z);
  end
endmodule
