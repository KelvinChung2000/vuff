module m;
  initial begin
    x = 1;
    // dangling before end
  end
endmodule
// expected -----
module m;
  initial begin
    x = 1;
    // dangling before end
  end
endmodule
