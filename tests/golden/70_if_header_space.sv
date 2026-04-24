module m;
  logic a;
  initial begin
    if(x) a = 1;
    else if(y) a = 0;
  end
endmodule
// expected -----
module m;
  logic a;
  initial begin
    if (x) a = 1;
    else if (y) a = 0;
  end
endmodule
