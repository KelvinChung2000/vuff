// config: begin_style=allman
module m;
initial begin : lbl
x = 1;
end : lbl
endmodule
// expected -----
module m;
  initial
  begin : lbl
    x = 1;
  end : lbl
endmodule
