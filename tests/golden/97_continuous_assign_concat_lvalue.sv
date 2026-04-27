module m;
  assign  {hi  ,  lo}  =  bus;
  assign {a, b, c} = {x, y, z};
endmodule
// expected -----
module m;
  assign {hi, lo} = bus;
  assign {a, b, c} = {x, y, z};
endmodule
