module m;
initial fork
a = 1;
b = 2;
join
endmodule
// expected -----
module m;
  initial fork
    a = 1;
    b = 2;
  join
endmodule
